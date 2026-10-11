//! Builds the unsigned `GenericResponse` for the open request, from values
//! that Rust read from the verified request (ADR 0002). Fields follow Verus
//! Mobile `prepareGenericResponseForSigning`.

use crate::core::crypto::verus_id_signature::{
    encode_compact_i_address, write_compact_size, write_var_slice, write_varint,
    VERIFIABLE_SIGNATURE_FLAG_HAS_SYSTEM,
};
use crate::types::WalletError;

use super::details::{self, DetailKind};
use super::request::{
    ParsedRequest, FLAG_HAS_CREATED_AT, FLAG_HAS_REQUEST_ID, FLAG_IS_TESTNET, FLAG_MULTI_DETAILS,
    FLAG_SIGNED,
};
use super::wire::invalid;

const ORDINAL_AUTHENTICATION_RESPONSE: usize = 3;
const ORDINAL_IDENTITY_UPDATE_RESPONSE: usize = 5;

const RESPONSE_FLAG_HAS_REQUEST_HASH: u64 = 128;
const RESPONSE_FLAG_HAS_HANDLED_BY: u64 = 256;
const AUTH_RESPONSE_FLAG_HAS_REQUEST_ID: u64 = 1;
const UPDATE_RESPONSE_FLAG_CONTAINS_TXID: u64 = 1;
const UPDATE_RESPONSE_FLAG_CONTAINS_REQUEST_ID: u64 = 2;

const SIGNATURE_DATA_VERSION: u64 = 0;
const SIGNATURE_VERSION_V2: usize = 2;
const HASH_TYPE_SHA256: usize = 5;
/// `VERUS_MOBILE_GENERIC_REQUEST_HANDLER_ID`. Desktop answers as a Verus
/// wallet, so it reports the same handler as Verus Mobile.
const HANDLED_BY_VERUS: usize = 0;

/// What the wallet did for the request, as the response reports it.
#[derive(Debug, Default, Clone)]
pub(crate) struct ResponseOutcome {
    /// The identity update or invoice payment broadcast after Approval.
    pub txid: Option<String>,
}

fn ordinal_detail(ordinal: usize, data: &[u8]) -> Vec<u8> {
    let mut detail = Vec::new();
    write_compact_size(&mut detail, ordinal);
    write_varint(&mut detail, 1);
    write_var_slice(&mut detail, data);
    detail
}

fn authentication_response(request_id: Option<&str>) -> Result<Vec<u8>, WalletError> {
    let mut data = Vec::new();
    match request_id {
        Some(request_id) => {
            write_varint(&mut data, AUTH_RESPONSE_FLAG_HAS_REQUEST_ID);
            data.extend_from_slice(&encode_compact_i_address(request_id)?);
        }
        None => write_varint(&mut data, 0),
    }
    Ok(ordinal_detail(ORDINAL_AUTHENTICATION_RESPONSE, &data))
}

fn identity_update_response(request_id: Option<&str>, txid: &str) -> Result<Vec<u8>, WalletError> {
    let mut txid_bytes = hex::decode(txid.trim()).map_err(|_| WalletError::OperationFailed)?;
    if txid_bytes.len() != 32 {
        return Err(WalletError::OperationFailed);
    }
    txid_bytes.reverse();

    let mut flags = UPDATE_RESPONSE_FLAG_CONTAINS_TXID;
    if request_id.is_some() {
        flags |= UPDATE_RESPONSE_FLAG_CONTAINS_REQUEST_ID;
    }
    let mut data = Vec::new();
    write_varint(&mut data, flags);
    if let Some(request_id) = request_id {
        data.extend_from_slice(&encode_compact_i_address(request_id)?);
    }
    data.extend_from_slice(&txid_bytes);
    Ok(ordinal_detail(ORDINAL_IDENTITY_UPDATE_RESPONSE, &data))
}

/// The response details, in request order. A sign-in response echoes the
/// request ID when the request has one, else the sign-in detail's own ID
/// (`AuthenticationRequestInfo.js`). Other details echo their own ID.
pub(crate) fn response_details(
    request: &ParsedRequest,
    kinds: &[DetailKind],
    outcome: &ResponseOutcome,
) -> Result<Vec<Vec<u8>>, WalletError> {
    // Verus Mobile sends no sign-in response next to an encryption key request.
    let has_app_encryption = kinds.contains(&DetailKind::AppEncryption);
    let mut out = Vec::new();
    for (detail, kind) in request.details.iter().zip(kinds) {
        match kind {
            DetailKind::Authentication if !has_app_encryption => {
                let auth = details::parse_authentication(&detail.data, request.is_testnet)?;
                let echo = request.request_id.as_deref().or(auth.request_id.as_deref());
                out.push(authentication_response(echo)?);
            }
            DetailKind::IdentityUpdate => {
                let update = details::parse_identity_update(&detail.data, request.is_testnet)?;
                let txid = outcome
                    .txid
                    .as_deref()
                    .ok_or(WalletError::GenericRequestInvalidEnvelope)?;
                out.push(identity_update_response(
                    update.request_id.as_deref(),
                    txid,
                )?);
            }
            // New VerusID and invoices add no response detail, as on mobile.
            DetailKind::Authentication | DetailKind::ProvisionIdentity | DetailKind::Invoice => {}
            _ => {
                return Err(WalletError::GenericRequestRejected(
                    "unsupportedDetail".into(),
                ))
            }
        }
    }
    Ok(out)
}

pub(crate) struct ResponseSigner<'a> {
    pub system_id: &'a str,
    pub identity_id: &'a str,
}

/// The unsigned response envelope. Its signature slot is empty; the caller
/// signs the envelope hash and fills it.
pub(crate) fn build_unsigned_response(
    request: &ParsedRequest,
    details: Vec<Vec<u8>>,
    signer: &ResponseSigner<'_>,
    created_at: u64,
) -> Result<Vec<u8>, WalletError> {
    if details.is_empty() {
        return Err(invalid());
    }
    let mut flags = FLAG_SIGNED
        | FLAG_HAS_CREATED_AT
        | RESPONSE_FLAG_HAS_REQUEST_HASH
        | RESPONSE_FLAG_HAS_HANDLED_BY;
    if request.is_testnet {
        flags |= FLAG_IS_TESTNET;
    }
    if request.request_id.is_some() {
        flags |= FLAG_HAS_REQUEST_ID;
    }
    if details.len() > 1 {
        flags |= FLAG_MULTI_DETAILS;
    }

    let mut out = Vec::new();
    write_compact_size(&mut out, 1);
    write_compact_size(&mut out, flags as usize);

    write_varint(&mut out, SIGNATURE_DATA_VERSION);
    write_compact_size(&mut out, VERIFIABLE_SIGNATURE_FLAG_HAS_SYSTEM as usize);
    write_compact_size(&mut out, SIGNATURE_VERSION_V2);
    write_compact_size(&mut out, HASH_TYPE_SHA256);
    out.extend_from_slice(&encode_compact_i_address(signer.system_id)?);
    out.extend_from_slice(&encode_compact_i_address(signer.identity_id)?);
    write_var_slice(&mut out, &[]);

    if let Some(request_id) = request.request_id.as_deref() {
        out.extend_from_slice(&encode_compact_i_address(request_id)?);
    }
    write_compact_size(&mut out, created_at as usize);

    if details.len() > 1 {
        write_compact_size(&mut out, details.len());
    }
    for detail in &details {
        out.extend_from_slice(detail);
    }

    write_compact_size(&mut out, HASH_TYPE_SHA256);
    write_var_slice(&mut out, &request.request_hash);
    write_compact_size(&mut out, HANDLED_BY_VERUS);
    Ok(out)
}
