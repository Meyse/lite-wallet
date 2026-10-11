//! Parses a whole `GenericRequest` envelope, signed or unsigned, including the
//! fields after the details: response addresses, the address to encrypt the
//! response to, and the preferred handler.

use crate::core::crypto::verus_id_signature::{
    parse_verifiable_signature_data, ParsedVerifiableSignatureData,
};
use crate::types::WalletError;

use super::names::{fqn_to_id, i_address, root_system_name, sha256};
use super::wire::{invalid, Reader};

const ENVELOPE_VERSION: u64 = 1;

pub(crate) const FLAG_SIGNED: u64 = 1;
pub(crate) const FLAG_HAS_REQUEST_ID: u64 = 2;
pub(crate) const FLAG_HAS_CREATED_AT: u64 = 4;
pub(crate) const FLAG_MULTI_DETAILS: u64 = 8;
pub(crate) const FLAG_IS_TESTNET: u64 = 16;
pub(crate) const FLAG_HAS_SALT: u64 = 32;
pub(crate) const FLAG_HAS_APP_OR_DELEGATED_ID: u64 = 64;
pub(crate) const FLAG_HAS_RESPONSE_URIS: u64 = 128;
pub(crate) const FLAG_HAS_ENCRYPT_RESPONSE_TO_ADDRESS: u64 = 256;
pub(crate) const FLAG_HAS_PREFERRED_HANDLER: u64 = 512;
pub(crate) const FLAG_DETAILS_ARE_ENCRYPTED: u64 = 1024;

const ORDINAL_RESERVED_I_ADDRESS_KEY: u64 = 102;
const ORDINAL_RESERVED_TEXT_KEY: u64 = 103;
const ORDINAL_RESERVED_ID_OR_CURRENCY_KEY: u64 = 104;

const RESPONSE_URI_TYPE_POST: u64 = 1;
const RESPONSE_URI_TYPE_REDIRECT: u64 = 2;

pub(crate) const COMPACT_ADDRESS_TYPE_FQN: u64 = 1;
pub(crate) const COMPACT_ADDRESS_TYPE_I_ADDRESS: u64 = 2;
pub(crate) const COMPACT_ADDRESS_TYPE_X_ADDRESS: u64 = 3;

/// A `CompactAddressObject`: an i-address, an x-address, or a fully qualified
/// name that is stored without its root system.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CompactAddress {
    IAddress([u8; 20]),
    XAddress([u8; 20]),
    Name(String),
}

impl CompactAddress {
    pub(crate) fn read(reader: &mut Reader<'_>) -> Result<Self, WalletError> {
        let _version = reader.compact_size()?;
        match reader.compact_size()? {
            COMPACT_ADDRESS_TYPE_I_ADDRESS => Ok(Self::IAddress(reader.hash160()?)),
            COMPACT_ADDRESS_TYPE_X_ADDRESS => Ok(Self::XAddress(reader.hash160()?)),
            COMPACT_ADDRESS_TYPE_FQN => Ok(Self::Name(reader.utf8()?)),
            _ => Err(invalid()),
        }
    }

    /// The identity this address names. A data key ("ns::name") or an
    /// x-address names no identity.
    pub(crate) fn identity_hash(&self, is_testnet: bool) -> Option<[u8; 20]> {
        match self {
            Self::IAddress(hash) => Some(*hash),
            Self::XAddress(_) => None,
            Self::Name(name) if name.contains("::") => None,
            Self::Name(name) => fqn_to_id(name, root_system_name(is_testnet)),
        }
    }

    pub(crate) fn identity_address(&self, is_testnet: bool) -> Result<String, WalletError> {
        self.identity_hash(is_testnet)
            .map(|hash| i_address(&hash))
            .ok_or_else(invalid)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ResponseUriKind {
    Post,
    Redirect,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ResponseUri {
    pub kind: ResponseUriKind,
    pub uri: String,
}

#[derive(Debug, Clone)]
pub(crate) struct RequestDetail {
    pub ordinal: u64,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone)]
pub(crate) struct ParsedRequest {
    pub flags: u64,
    pub is_testnet: bool,
    pub signature: Option<ParsedVerifiableSignatureData>,
    pub request_id: Option<String>,
    pub created_at: Option<u64>,
    pub app_or_delegated_id: Option<String>,
    pub details: Vec<RequestDetail>,
    pub response_uris: Vec<ResponseUri>,
    pub encrypt_response_to: Option<[u8; 43]>,
    pub preferred_handler: Option<u64>,
    /// SHA-256 of the request without its signature: `getRawDataSha256()`,
    /// which the response returns as its request hash.
    pub request_hash: [u8; 32],
}

impl ParsedRequest {
    pub(crate) fn is_signed(&self) -> bool {
        self.signature.is_some()
    }

    #[cfg(test)]
    pub(crate) fn ordinals(&self) -> Vec<u64> {
        self.details.iter().map(|detail| detail.ordinal).collect()
    }
}

fn read_detail(reader: &mut Reader<'_>) -> Result<RequestDetail, WalletError> {
    let ordinal = reader.compact_size()?;
    // Details keyed by a VDXF key instead of a registered ordinal are not a
    // request type that any wallet answers; skip the key so acceptance can
    // reject them by ordinal.
    match ordinal {
        ORDINAL_RESERVED_I_ADDRESS_KEY => {
            reader.hash160()?;
        }
        ORDINAL_RESERVED_TEXT_KEY | ORDINAL_RESERVED_ID_OR_CURRENCY_KEY => {
            reader.var_slice()?;
        }
        _ => {}
    }
    let _version = reader.varint()?;
    let data = reader.var_slice()?.to_vec();
    Ok(RequestDetail { ordinal, data })
}

pub(crate) fn parse_request(bytes: &[u8]) -> Result<ParsedRequest, WalletError> {
    let mut reader = Reader::new(bytes);
    if reader.compact_size()? != ENVELOPE_VERSION {
        return Err(invalid());
    }
    let flags = reader.compact_size()?;
    let is_testnet = flags & FLAG_IS_TESTNET != 0;
    let prefix_end = reader.position();

    let signature = if flags & FLAG_SIGNED != 0 {
        let (signature, end) = parse_verifiable_signature_data(bytes, prefix_end, is_testnet)?;
        reader.take(end - prefix_end)?;
        Some(signature)
    } else {
        None
    };
    let signature_end = reader.position();

    let request_id = if flags & FLAG_HAS_REQUEST_ID != 0 {
        Some(CompactAddress::read(&mut reader)?.identity_address(is_testnet)?)
    } else {
        None
    };
    let created_at = if flags & FLAG_HAS_CREATED_AT != 0 {
        Some(reader.compact_size()?)
    } else {
        None
    };
    if flags & FLAG_HAS_SALT != 0 {
        reader.var_slice()?;
    }
    let app_or_delegated_id = if flags & FLAG_HAS_APP_OR_DELEGATED_ID != 0 {
        Some(CompactAddress::read(&mut reader)?.identity_address(is_testnet)?)
    } else {
        None
    };

    let details = if flags & FLAG_MULTI_DETAILS != 0 && flags & FLAG_DETAILS_ARE_ENCRYPTED == 0 {
        let count = reader.count()?;
        let mut details = Vec::with_capacity(count);
        for _ in 0..count {
            details.push(read_detail(&mut reader)?);
        }
        details
    } else {
        vec![read_detail(&mut reader)?]
    };

    let mut response_uris = Vec::new();
    if flags & FLAG_HAS_RESPONSE_URIS != 0 {
        let count = reader.count()?;
        for _ in 0..count {
            let kind = match reader.compact_size()? {
                RESPONSE_URI_TYPE_POST => ResponseUriKind::Post,
                RESPONSE_URI_TYPE_REDIRECT => ResponseUriKind::Redirect,
                _ => return Err(invalid()),
            };
            response_uris.push(ResponseUri {
                kind,
                uri: reader.utf8()?,
            });
        }
    }
    let encrypt_response_to = if flags & FLAG_HAS_ENCRYPT_RESPONSE_TO_ADDRESS != 0 {
        Some(reader.take(43)?.try_into().map_err(|_| invalid())?)
    } else {
        None
    };
    let preferred_handler = if flags & FLAG_HAS_PREFERRED_HANDLER != 0 {
        Some(reader.compact_size()?)
    } else {
        None
    };
    reader.finish()?;

    let mut unsigned = Vec::with_capacity(bytes.len());
    unsigned.extend_from_slice(&bytes[..prefix_end]);
    unsigned.extend_from_slice(&bytes[signature_end..]);

    Ok(ParsedRequest {
        flags,
        is_testnet,
        signature,
        request_id,
        created_at,
        app_or_delegated_id,
        details,
        response_uris,
        encrypt_response_to,
        preferred_handler,
        request_hash: sha256(&unsigned),
    })
}

pub(crate) fn parse_request_hex(hex_value: &str) -> Result<ParsedRequest, WalletError> {
    let bytes = hex::decode(hex_value.trim()).map_err(|_| invalid())?;
    parse_request(&bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::generic_request::details::{
        parse_authentication, parse_identity_update, ORDINAL_AUTHENTICATION,
        ORDINAL_IDENTITY_UPDATE,
    };
    use serde_json::Value;

    fn case(name: &str) -> Value {
        let raw = include_str!("../../../../src/lib/genericRequest/fixtures/request-vectors.json");
        let parsed: Value = serde_json::from_str(raw).expect("vectors parse");
        parsed["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|case| case["name"] == name)
            .unwrap_or_else(|| panic!("no vector {name}"))
            .clone()
    }

    fn parse(name: &str) -> (ParsedRequest, Value) {
        let case = case(name);
        let request = parse_request_hex(case["hex"].as_str().unwrap()).expect("request parses");
        assert_eq!(
            hex::encode(request.request_hash),
            case["requestHash"].as_str().unwrap(),
            "{name}: request hash differs from getRawDataSha256()"
        );
        (request, case["expect"].clone())
    }

    #[test]
    fn signed_request_fields_match_the_library() {
        let (request, expect) = parse("auth");
        assert!(request.is_signed());
        assert!(request.is_testnet);
        assert_eq!(request.ordinals(), vec![ORDINAL_AUTHENTICATION]);
        assert_eq!(request.request_id.as_deref(), expect["requestId"].as_str());
        assert_eq!(
            request.signature.as_ref().unwrap().signer_identity_id,
            expect["signerIdentity"].as_str().unwrap()
        );
        let uris = request
            .response_uris
            .iter()
            .map(|uri| (uri.kind, uri.uri.as_str()))
            .collect::<Vec<_>>();
        assert_eq!(
            uris,
            vec![
                (
                    ResponseUriKind::Post,
                    "https://lumen.example/verus/response"
                ),
                (
                    ResponseUriKind::Redirect,
                    "https://lumen.example/verus/return"
                ),
            ]
        );
        let auth = parse_authentication(&request.details[0].data, true).unwrap();
        assert_eq!(auth.request_id.as_deref(), expect["authRequestId"].as_str());
        assert_eq!(auth.expiry_time, expect["authExpiryTime"].as_u64());
    }

    #[test]
    fn sign_in_constraints_are_read_in_order() {
        let (request, expect) = parse("auth-constraints-no-ids");
        assert!(request.request_id.is_none());
        let auth = parse_authentication(&request.details[0].data, true).unwrap();
        assert!(auth.request_id.is_none());
        let constraints = auth
            .constraints
            .iter()
            .map(|constraint| serde_json::json!([constraint.kind, constraint.identity]))
            .collect::<Vec<_>>();
        assert_eq!(Value::Array(constraints), expect["authConstraints"]);
    }

    #[test]
    fn identity_update_fields_match_the_library() {
        let (request, expect) = parse("auth-update");
        assert_eq!(request.details[1].ordinal, ORDINAL_IDENTITY_UPDATE);
        let update = parse_identity_update(&request.details[1].data, true).unwrap();
        let expected = &expect["update"];
        assert_eq!(update.request_id.as_deref(), expected["requestId"].as_str());
        assert_eq!(update.expiry_height, expected["expiryHeight"].as_u64());
        assert_eq!(update.system_id.as_deref(), expected["systemId"].as_str());
        assert_eq!(
            update.target_identity(true).as_deref(),
            expected["target"].as_str()
        );
        let identity = &update.identity;
        assert_eq!(identity.name, expected["name"].as_str().unwrap());
        assert_eq!(identity.parent.as_deref(), expected["parent"].as_str());
        assert_eq!(
            identity.primary_addresses.as_deref(),
            Some(
                &[expected["primaryAddresses"][0]
                    .as_str()
                    .unwrap()
                    .to_string()][..]
            )
        );
        assert_eq!(
            identity.minimum_signatures,
            expected["minimumSignatures"]
                .as_u64()
                .map(|value| value as u32)
        );
        assert_eq!(
            identity.revocation.as_deref(),
            expected["revocation"].as_str()
        );
        assert_eq!(identity.recovery.as_deref(), expected["recovery"].as_str());
        assert_eq!(
            identity.private_addresses.as_ref().unwrap()[0],
            expected["privateAddress"].as_str().unwrap()
        );
        let keys = identity
            .content_multimap
            .as_ref()
            .unwrap()
            .iter()
            .map(|(key, _)| key.as_str())
            .collect::<Vec<_>>();
        assert_eq!(keys, vec![expected["contentKeys"][0].as_str().unwrap()]);
        assert!(!update.contains_sign_data);
    }

    #[test]
    fn share_data_fields_match_the_library() {
        let (request, expect) = parse("auth-credential");
        let user_data =
            crate::core::generic_request::details::parse_user_data(&request.details[1].data, true)
                .unwrap();
        let expected = &expect["userData"];
        assert!(user_data.is_valid());
        assert_eq!(
            user_data.request_id.as_deref(),
            expected["requestId"].as_str()
        );
        assert_eq!(user_data.signer.as_deref(), expected["signer"].as_str());
        let keys = user_data
            .search_keys
            .iter()
            .map(|(key, _)| Value::String(key.clone()))
            .collect::<Vec<_>>();
        assert_eq!(Value::Array(keys), expected["keys"]);
    }

    #[test]
    fn credentials_in_an_update_keep_their_bytes() {
        let (request, expect) = parse("update-credentials");
        let update = parse_identity_update(&request.details[0].data, true).unwrap();
        let entries = update.identity.content_multimap.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(
            entries[0].0,
            crate::core::generic_request::credentials::IDENTITY_CREDENTIAL_KEY
        );
        let values = entries[0]
            .1
            .iter()
            .map(|value| Value::String(hex::encode(value)))
            .collect::<Vec<_>>();
        assert_eq!(Value::Array(values), expect["credentialValues"]);
    }

    #[test]
    fn a_signer_named_by_its_full_name_resolves_to_its_i_address() {
        let (request, expect) = parse("fqn-signer");
        assert_eq!(
            request.signature.unwrap().signer_identity_id,
            expect["signerIdentity"].as_str().unwrap()
        );
    }

    #[test]
    fn envelope_tail_fields_are_read() {
        let (request, expect) = parse("delegated-app");
        assert_eq!(
            request.app_or_delegated_id.as_deref(),
            expect["appOrDelegatedId"].as_str()
        );
        let (request, expect) = parse("encrypt-to");
        let raw = request.encrypt_response_to.expect("encrypt-to address");
        assert_eq!(
            crate::core::generic_request::names::sapling_address(&raw).unwrap(),
            expect["encryptTo"].as_str().unwrap()
        );
    }

    #[test]
    fn unsigned_requests_parse_without_a_signature() {
        let (request, _) = parse("invoice-unsigned");
        assert!(!request.is_signed());
        assert!(request.request_id.is_none());
    }

    #[test]
    fn trailing_bytes_are_rejected() {
        let case = case("auth");
        let hex_value = format!("{}00", case["hex"].as_str().unwrap());
        assert!(parse_request_hex(&hex_value).is_err());
    }
}
