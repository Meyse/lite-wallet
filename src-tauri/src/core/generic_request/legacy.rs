//! Legacy links that Verus Mobile still answers (`src/sagas/deeplink.js` at
//! `dev-asf-parent`): a login consent request, and VerusPay invoices at
//! versions 3 and 4. They arrive as `<scheme>://x-callback-url/<key>/...`.
//!
//! A legacy invoice becomes the invoice detail of a request (version 4), so
//! the invoice commands pay it as they pay any invoice. A login consent request
//! has its own response (`LoginConsentResponse`).

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use sha2::{Digest, Sha256};

use crate::core::crypto::verus_id_signature::{
    compute_verus_data_identity_hash, write_compact_size, write_var_slice, write_varint,
    VERUS_DATA_SIGNATURE_PREFIX,
};
use crate::types::WalletError;

use super::acceptance::Rejection;
use super::details::{
    INVOICE_ACCEPTS_ANY_AMOUNT, INVOICE_ACCEPTS_ANY_DESTINATION, INVOICE_ACCEPTS_CONVERSION,
    INVOICE_ACCEPTS_NON_VERUS_SYSTEMS, INVOICE_EXPIRES,
};
use super::names::{decode_i_address, i_address, root_system_id, sha256};
use super::wire::{invalid, Reader};

pub(crate) const LOGIN_CONSENT_REQUEST_KEY: &str = "i3dQmgjq8L8XFGQUrs9Gpo8zvPWqs1KMtV";
/// Also the query parameter that carries a response to a redirect address.
pub(crate) const LOGIN_CONSENT_RESPONSE_KEY: &str = "i5fvfsaTFKtrHCPYQHTXRaXcyxHmJMxTMe";
const LOGIN_CONSENT_CHALLENGE_KEY: &str = "i5maLnB62WmKKXFZniqDRU1JiC2Hd1xpVb";
const LOGIN_CONSENT_DECISION_KEY: &str = "iQP5eKQaYDV3FFXsq7276LyWxk4ttjuSdm";
const LOGIN_CONSENT_CONTEXT_KEY: &str = "iBMochrKPSQfua5yZYWyd6p4QnREakqU44";
pub(crate) const LOGIN_CONSENT_REDIRECT_KEY: &str = "iDXvHYhRpWcoARCEYeLv8GwkVdrbvSFuam";
pub(crate) const LOGIN_CONSENT_WEBHOOK_KEY: &str = "iSaBWByu4zqhEZ6HQmFxvfR1HyiFuhnJfL";
/// `IDENTITY_AUTH_SIG_VDXF_KEY`, which `LOGIN_CONSENT_RESPONSE_SIG_VDXF_KEY`
/// equals.
const IDENTITY_AUTH_SIG_KEY: &str = "iPi1DPgDDu7hP1mAp5xJ8rHBWwXSzc6yA8";
/// `provisioning_info` keys Verus Mobile reads (`ProvisionIdentityForm.js`).
const PROVISIONING_WEBHOOK_KEY: &str = "iMiXw4BuL4iESPqz6fvJ4rHbDg1SvVKLnc";
const ID_ADDRESS_KEY: &str = "i3a3M9n7uVtRYv1vhjmyb4DxY825AVAwic";
const ID_SYSTEMID_KEY: &str = "iMZTNkNBgBXNHkMLipQw9wQb56pxBSEp3k";
const ID_PARENT_KEY: &str = "i6aJSTKfNiDZ4rPxj1pPh4Y8xDmh1GqYm9";
const ID_FULLYQUALIFIEDNAME_KEY: &str = "iCQ5gYekWs5DaXiBN7YfoDfNWT3VtpUwVq";
/// `IDENTITY_VIEW`, the only permission Verus Mobile grants.
const IDENTITY_VIEW: &str = "iLUrA89mDKnwxZcMiPadfNB9TLp58A2TKU";
pub(crate) const VERUSPAY_INVOICE_KEY: &str = "iEETy7La3FTN2Sd2hNRgepek5S8x8eeUeQ";
const CALLBACK_HOST: &str = "x-callback-url";
const VDXF_OBJECT_VERSION: u64 = 1;
const VERUSPAY_VERSION_SIGNED: u64 = 0x8000_0000;
const VERUSPAY_VERSION_3: u64 = 3;
const VERUSPAY_VERSION_4: u64 = 4;
/// Invoice flags that version 3 knows; later flags mean nothing there.
const INVOICE_VERSION_3_FLAGS: u64 = 0xff;

/// A legacy link, decoded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum LegacyLink {
    /// The login consent request, with its VDXF key.
    Login(Vec<u8>),
    /// The invoice without its VDXF key: version, then the data.
    Invoice(Vec<u8>),
}

/// Whether `text` is a link in the legacy form. `verus://` links are always
/// requests, as on mobile.
pub(crate) fn is_legacy_link(text: &str) -> bool {
    let text = text.trim();
    let scheme = text.split("://").next().unwrap_or_default();
    text.contains("://x-callback-url/") && !matches!(scheme, "verus" | "verus0")
}

fn decode_base64url(value: &str) -> Result<Vec<u8>, WalletError> {
    URL_SAFE_NO_PAD
        .decode(value.trim().trim_end_matches('='))
        .map_err(|_| invalid())
}

pub(crate) fn parse_legacy_link(link: &str) -> Result<LegacyLink, WalletError> {
    if !is_legacy_link(link) {
        return Err(invalid());
    }
    let url = reqwest::Url::parse(link.trim()).map_err(|_| invalid())?;
    if url.host_str() != Some(CALLBACK_HOST) {
        return Err(invalid());
    }
    let mut segments = url.path_segments().ok_or_else(invalid)?;
    match segments.next() {
        Some(LOGIN_CONSENT_REQUEST_KEY) => {
            let value = url
                .query_pairs()
                .find(|(key, _)| key == LOGIN_CONSENT_REQUEST_KEY)
                .map(|(_, value)| value.into_owned())
                .ok_or_else(invalid)?;
            Ok(LegacyLink::Login(decode_base64url(&value)?))
        }
        Some(VERUSPAY_INVOICE_KEY) => Ok(LegacyLink::Invoice(decode_base64url(
            segments.next().ok_or_else(invalid)?,
        )?)),
        _ => Err(Rejection::UnknownDetail.into()),
    }
}

/// A VDXF object: its key, version, and data.
struct VdxfObject<'a> {
    key: String,
    data: &'a [u8],
    bytes: &'a [u8],
}

fn read_object<'a>(reader: &mut Reader<'a>) -> Result<VdxfObject<'a>, WalletError> {
    let start = reader.position();
    let key = i_address(&reader.hash160()?);
    let _version = reader.varint()?;
    let data = reader.var_slice()?;
    Ok(VdxfObject {
        key,
        data,
        bytes: reader.since(start),
    })
}

fn read_objects<'a>(reader: &mut Reader<'a>) -> Result<Vec<VdxfObject<'a>>, WalletError> {
    let count = reader.count()?;
    (0..count).map(|_| read_object(reader)).collect()
}

fn write_object(out: &mut Vec<u8>, key: &str, data: &[u8]) -> Result<(), WalletError> {
    out.extend_from_slice(&decode_i_address(key).ok_or_else(invalid)?);
    write_varint(out, VDXF_OBJECT_VERSION);
    write_var_slice(out, data);
    Ok(())
}

// --- Login consent -----------------------------------------------------------

/// `LoginConsentRequest`, as the wallet reads it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LoginConsentRequest {
    /// The request with its VDXF key; the response embeds it unchanged.
    pub bytes: Vec<u8>,
    pub system_id: [u8; 20],
    pub signing_id: [u8; 20],
    /// The requester's identity signature.
    pub signature: Vec<u8>,
    /// SHA-256 of the challenge with its VDXF key, which the requester signs.
    pub challenge_sha256: [u8; 32],
    pub challenge_id: [u8; 20],
    pub created_at: u64,
    pub permissions: Vec<String>,
    /// Each redirect address with its kind (webhook or redirect key).
    pub redirect_uris: Vec<(String, String)>,
    pub context_entries: usize,
    /// A new VerusID the requester offers ("Request VerusID" on mobile).
    pub provisioning: Option<LegacyProvisioning>,
}

/// The `provisioning_info` of a login consent request, read as Verus Mobile
/// reads it: the first entry of each key (`ProvisionIdentityForm.js`). It
/// offers a new VerusID only with a provisioning webhook.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LegacyProvisioning {
    pub webhook: String,
    pub system_id: Option<String>,
    pub parent_id: Option<String>,
    pub identity_id: Option<String>,
    pub fully_qualified_name: Option<String>,
}

/// `Utf8OrBase58Object`: an ID address, parent, or system is a hash; any
/// other entry is text.
fn read_provisioning(
    objects: &[VdxfObject<'_>],
) -> Result<Option<LegacyProvisioning>, WalletError> {
    let first = |key: &str| objects.iter().find(|object| object.key == key);
    let text = |key: &str| {
        first(key)
            .map(|object| {
                std::str::from_utf8(object.data)
                    .map(|value| value.trim().to_string())
                    .map_err(|_| invalid())
            })
            .transpose()
    };
    let address = |key: &str| {
        first(key)
            .map(|object| {
                <[u8; 20]>::try_from(object.data)
                    .map(|hash| i_address(&hash))
                    .map_err(|_| invalid())
            })
            .transpose()
    };
    let Some(webhook) = text(PROVISIONING_WEBHOOK_KEY)?.filter(|uri| !uri.is_empty()) else {
        return Ok(None);
    };
    Ok(Some(LegacyProvisioning {
        webhook,
        system_id: address(ID_SYSTEMID_KEY)?,
        parent_id: address(ID_PARENT_KEY)?,
        identity_id: address(ID_ADDRESS_KEY)?,
        fully_qualified_name: text(ID_FULLYQUALIFIEDNAME_KEY)?.filter(|name| !name.is_empty()),
    }))
}

fn read_hash160_slice(reader: &mut Reader<'_>) -> Result<[u8; 20], WalletError> {
    reader.var_slice()?.try_into().map_err(|_| invalid())
}

/// `LoginConsentRequest.fromBuffer`, with the challenge as
/// `LoginConsentChallenge.fromDataBuffer` reads it.
pub(crate) fn parse_login_consent(bytes: &[u8]) -> Result<LoginConsentRequest, WalletError> {
    let mut outer = Reader::new(bytes);
    let request = read_object(&mut outer)?;
    outer.finish()?;
    if request.key != LOGIN_CONSENT_REQUEST_KEY {
        return Err(invalid());
    }

    let mut body = Reader::new(request.data);
    let system_id = body.hash160()?;
    let signing_id = body.hash160()?;
    let signature = read_object(&mut body)?;
    let challenge = read_object(&mut body)?;
    body.finish()?;
    if challenge.key != LOGIN_CONSENT_CHALLENGE_KEY {
        return Err(invalid());
    }

    let mut reader = Reader::new(challenge.data);
    let challenge_id = read_hash160_slice(&mut reader)?;
    let created_at = reader.u64_le()?;
    let _salt = reader.var_slice()?;
    let _skip = reader.u8()?;
    let _session_id = reader.var_slice()?;
    let permissions = read_objects(&mut reader)?
        .into_iter()
        .map(|object| object.key)
        .collect();
    // The library refuses an audience or alternative factors.
    if reader.count()? != 0 {
        return Err(invalid());
    }
    let _subjects = read_objects(&mut reader)?;
    let provisioning = read_provisioning(&read_objects(&mut reader)?)?;
    if reader.count()? != 0 {
        return Err(invalid());
    }
    let _attestations = read_objects(&mut reader)?;
    let redirect_uris = read_objects(&mut reader)?
        .into_iter()
        .map(|object| {
            let uri = std::str::from_utf8(object.data).map_err(|_| invalid())?;
            Ok((object.key, uri.to_string()))
        })
        .collect::<Result<Vec<_>, WalletError>>()?;
    let context = read_object(&mut reader)?;
    reader.finish()?;
    let context_entries = if context.data.is_empty() {
        0
    } else {
        Reader::new(context.data).count()?
    };

    Ok(LoginConsentRequest {
        bytes: bytes.to_vec(),
        system_id,
        signing_id,
        signature: signature.data.to_vec(),
        challenge_sha256: sha256(challenge.bytes),
        challenge_id,
        created_at,
        permissions,
        redirect_uris,
        context_entries,
        provisioning,
    })
}

/// Verus Mobile's rules (`DeepLink.js`): no context, only the identity view
/// permission, and the open wallet's network.
pub(crate) fn check_login_consent(
    request: &LoginConsentRequest,
    wallet_is_testnet: bool,
) -> Result<(), Rejection> {
    if request.context_entries > 0 {
        return Err(Rejection::LoginContext);
    }
    if request.permissions.is_empty()
        || request
            .permissions
            .iter()
            .any(|permission| permission != IDENTITY_VIEW)
    {
        return Err(Rejection::LoginPermission);
    }
    if i_address(&request.system_id) != root_system_id(wallet_is_testnet) {
        return Err(Rejection::Network);
    }
    Ok(())
}

impl LoginConsentRequest {
    /// `getChallengeHash(height)`: Verus Mobile verifies version 2.
    pub(crate) fn challenge_hash(&self, signed_block_height: u32) -> [u8; 32] {
        compute_verus_data_identity_hash(
            self.system_id,
            self.signing_id,
            signed_block_height,
            self.challenge_sha256,
        )
    }

    /// The redirect address the response goes to: a redirect when the request
    /// has one, else a webhook (`LoginRequestComplete.js`; the last of each
    /// kind wins).
    pub(crate) fn delivery(&self) -> Option<(&str, &str)> {
        let last = |kind: &str| {
            self.redirect_uris
                .iter()
                .rev()
                .find(|(key, _)| key == kind)
                .map(|(key, uri)| (key.as_str(), uri.as_str()))
        };
        last(LOGIN_CONSENT_REDIRECT_KEY).or_else(|| last(LOGIN_CONSENT_WEBHOOK_KEY))
    }

    /// `LoginConsentDecision`: the challenge ID, the time, no salt, not
    /// skipped, no attestations, an empty context, and the request.
    pub(crate) fn decision(&self, created_at: u64) -> Result<Vec<u8>, WalletError> {
        let mut data = Vec::new();
        write_var_slice(&mut data, &self.challenge_id);
        data.extend_from_slice(&created_at.to_le_bytes());
        write_var_slice(&mut data, &[]);
        data.push(0);
        write_compact_size(&mut data, 0);
        write_object(&mut data, LOGIN_CONSENT_CONTEXT_KEY, &[0])?;
        data.extend_from_slice(&self.bytes);
        let mut out = Vec::new();
        write_object(&mut out, LOGIN_CONSENT_DECISION_KEY, &data)?;
        Ok(out)
    }
}

/// `getDecisionHash(height)` at version 2, for the responder.
pub(crate) fn decision_hash(
    system_id: [u8; 20],
    responder: [u8; 20],
    signed_block_height: u32,
    decision: &[u8],
) -> [u8; 32] {
    compute_verus_data_identity_hash(system_id, responder, signed_block_height, sha256(decision))
}

/// `LoginConsentResponse`, with its VDXF key.
pub(crate) fn login_response(
    system_id: [u8; 20],
    responder: [u8; 20],
    signature: &[u8],
    decision: &[u8],
) -> Result<Vec<u8>, WalletError> {
    let mut data = Vec::new();
    data.extend_from_slice(&system_id);
    data.extend_from_slice(&responder);
    write_object(&mut data, IDENTITY_AUTH_SIG_KEY, signature)?;
    data.extend_from_slice(decision);
    let mut out = Vec::new();
    write_object(&mut out, LOGIN_CONSENT_RESPONSE_KEY, &data)?;
    Ok(out)
}

/// The fields of a login response that its webhook body must repeat.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LoginResponseFields {
    pub system_id: [u8; 20],
    pub responder: [u8; 20],
    pub signature: Vec<u8>,
    pub decision_id: [u8; 20],
    pub created_at: u64,
}

pub(crate) fn parse_login_response(bytes: &[u8]) -> Result<LoginResponseFields, WalletError> {
    let mut outer = Reader::new(bytes);
    let response = read_object(&mut outer)?;
    outer.finish()?;
    if response.key != LOGIN_CONSENT_RESPONSE_KEY {
        return Err(invalid());
    }
    let mut body = Reader::new(response.data);
    let system_id = body.hash160()?;
    let responder = body.hash160()?;
    let signature = read_object(&mut body)?.data.to_vec();
    let decision = read_object(&mut body)?;
    body.finish()?;
    if decision.key != LOGIN_CONSENT_DECISION_KEY {
        return Err(invalid());
    }
    let mut reader = Reader::new(decision.data);
    let decision_id = read_hash160_slice(&mut reader)?;
    let created_at = reader.u64_le()?;
    Ok(LoginResponseFields {
        system_id,
        responder,
        signature,
        decision_id,
        created_at,
    })
}

/// Verus Mobile posts the response object as JSON (`axios.post(uri,
/// response)`); the UI builds that JSON with the library. The body must
/// repeat the signed response's system, responder, signature, decision ID,
/// and time.
pub(crate) fn check_webhook_body(
    response: &LoginResponseFields,
    body: &serde_json::Value,
) -> Result<(), WalletError> {
    use base64::engine::general_purpose::STANDARD;
    let text = |value: &serde_json::Value| value.as_str().map(str::to_string);
    let matches = text(&body["system_id"]) == Some(i_address(&response.system_id))
        && text(&body["signing_id"]) == Some(i_address(&response.responder))
        && text(&body["signature"]["signature"]) == Some(STANDARD.encode(&response.signature))
        && text(&body["decision"]["decision_id"]) == Some(i_address(&response.decision_id))
        && body["decision"]["created_at"].as_u64() == Some(response.created_at);
    if matches {
        Ok(())
    } else {
        Err(WalletError::GenericRequestRejected(
            "responseAddress".to_string(),
        ))
    }
}

// --- Invoices ----------------------------------------------------------------

/// The requester of a signed legacy invoice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LegacySigner {
    pub system_id: [u8; 20],
    pub signing_id: [u8; 20],
    pub signature: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LegacyInvoice {
    pub signer: Option<LegacySigner>,
    /// The details as the link encodes them, which a signature covers.
    pub details: Vec<u8>,
    /// The same details at version 4, as a request's invoice detail.
    pub details_v4: Vec<u8>,
}

/// `VerusPayInvoice.fromWalletDeeplinkUri`: the version (with the signed
/// flag), then the data: for a signed invoice the system, the signer, and its
/// signature, then the details.
pub(crate) fn parse_legacy_invoice(payload: &[u8]) -> Result<LegacyInvoice, WalletError> {
    let mut outer = Reader::new(payload);
    let version = outer.varint()?;
    let data = outer.var_slice()?;
    outer.finish()?;
    let signed = version & VERUSPAY_VERSION_SIGNED != 0;
    let version = version & !VERUSPAY_VERSION_SIGNED;
    if !(VERUSPAY_VERSION_3..=VERUSPAY_VERSION_4).contains(&version) {
        return Err(invalid());
    }

    let mut reader = Reader::new(data);
    let signer = if signed {
        let system_id = reader.hash160()?;
        let signing_id = reader.hash160()?;
        // A signature without its VDXF key.
        let _signature_version = reader.varint()?;
        let signature = reader.var_slice()?.to_vec();
        Some(LegacySigner {
            system_id,
            signing_id,
            signature,
        })
    } else {
        None
    };
    let details = reader.take(reader.remaining())?.to_vec();
    let details_v4 = if version == VERUSPAY_VERSION_3 {
        version_3_details_to_4(&details)?
    } else {
        details.clone()
    };
    super::details::parse_invoice(&details_v4)?;
    Ok(LegacyInvoice {
        signer,
        details,
        details_v4,
    })
}

/// Version 3 writes the flags, the amount, the expiry, and the slippage as
/// VARINTs; version 4 writes them as CompactSize values. Everything else is
/// the same.
fn version_3_details_to_4(details: &[u8]) -> Result<Vec<u8>, WalletError> {
    let mut reader = Reader::new(details);
    let mut out = Vec::with_capacity(details.len() + 8);
    let flags = reader.varint()? & INVOICE_VERSION_3_FLAGS;
    write_compact_size(&mut out, usize::try_from(flags).map_err(|_| invalid())?);
    let number = |reader: &mut Reader<'_>, out: &mut Vec<u8>| -> Result<(), WalletError> {
        let value = reader.varint()?;
        write_compact_size(out, usize::try_from(value).map_err(|_| invalid())?);
        Ok(())
    };
    if flags & INVOICE_ACCEPTS_ANY_AMOUNT == 0 {
        number(&mut reader, &mut out)?;
    }
    if flags & INVOICE_ACCEPTS_ANY_DESTINATION == 0 {
        let start = reader.position();
        super::details::read_transfer_destination(&mut reader)?;
        out.extend_from_slice(reader.since(start));
    }
    out.extend_from_slice(&reader.hash160()?);
    if flags & INVOICE_EXPIRES != 0 {
        number(&mut reader, &mut out)?;
    }
    if flags & INVOICE_ACCEPTS_CONVERSION != 0 {
        number(&mut reader, &mut out)?;
    }
    if flags & INVOICE_ACCEPTS_NON_VERUS_SYSTEMS != 0 {
        let count = reader.count()?;
        write_compact_size(&mut out, count);
        for _ in 0..count {
            out.extend_from_slice(&reader.hash160()?);
        }
    }
    reader.finish()?;
    Ok(out)
}

impl LegacyInvoice {
    /// `getDetailsHash(height, signatureVersion)`: a signed invoice binds the
    /// system, the height, and the signer; version 1 puts the prefix first.
    pub(crate) fn details_hash(&self, signed_block_height: u32, signature_version: u8) -> [u8; 32] {
        let details_sha256 = sha256(&self.details);
        let Some(signer) = &self.signer else {
            return details_sha256;
        };
        if signature_version == 1 {
            let mut hasher = Sha256::new();
            hasher.update(VERUS_DATA_SIGNATURE_PREFIX);
            hasher.update(signer.system_id);
            hasher.update(signed_block_height.to_le_bytes());
            hasher.update(signer.signing_id);
            hasher.update(details_sha256);
            return hasher.finalize().into();
        }
        compute_verus_data_identity_hash(
            signer.system_id,
            signer.signing_id,
            signed_block_height,
            details_sha256,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn legacy() -> Value {
        let raw = include_str!("../../../../src/lib/genericRequest/fixtures/request-vectors.json");
        serde_json::from_str::<Value>(raw).unwrap()["legacy"].clone()
    }

    fn login() -> LoginConsentRequest {
        match parse_legacy_link(legacy()["login"]["link"].as_str().unwrap()).unwrap() {
            LegacyLink::Login(bytes) => parse_login_consent(&bytes).unwrap(),
            other => panic!("expected a login link, got {other:?}"),
        }
    }

    #[test]
    fn a_login_link_reads_like_the_library() {
        let vectors = legacy();
        let expected = &vectors["login"];
        let request = login();
        assert_eq!(
            i_address(&request.challenge_id),
            expected["challengeId"].as_str().unwrap()
        );
        assert_eq!(
            i_address(&request.signing_id),
            expected["signingId"].as_str().unwrap()
        );
        assert_eq!(
            hex::encode(&request.signature),
            vectors["signature"].as_str().unwrap()
        );
        assert_eq!(request.permissions, vec![IDENTITY_VIEW.to_string()]);
        assert_eq!(
            hex::encode(request.challenge_hash(vectors["height"].as_u64().unwrap() as u32)),
            expected["challengeHash"].as_str().unwrap()
        );
        // A redirect address wins over a webhook, as on mobile.
        assert_eq!(
            request.delivery(),
            Some((
                LOGIN_CONSENT_REDIRECT_KEY,
                expected["redirect"].as_str().unwrap()
            ))
        );
        assert!(check_login_consent(&request, true).is_ok());
        assert_eq!(
            check_login_consent(&request, false),
            Err(Rejection::Network)
        );
    }

    #[test]
    fn a_login_can_offer_a_new_verusid_as_mobile_reads_it() {
        let vectors = legacy();
        let expected = &vectors["loginProvisioning"];
        let request = match parse_legacy_link(expected["link"].as_str().unwrap()).unwrap() {
            LegacyLink::Login(bytes) => parse_login_consent(&bytes).unwrap(),
            other => panic!("expected a login link, got {other:?}"),
        };
        let text = |key: &str| Some(expected[key].as_str().unwrap().to_string());
        assert_eq!(
            request.provisioning,
            Some(LegacyProvisioning {
                // The first webhook entry wins.
                webhook: text("webhook").unwrap(),
                system_id: text("systemId"),
                parent_id: text("parent"),
                identity_id: None,
                fully_qualified_name: text("fullyQualifiedName"),
            })
        );
        assert!(check_login_consent(&request, true).is_ok());
        assert_eq!(login().provisioning, None);
    }

    #[test]
    fn a_login_response_is_byte_for_byte_what_mobile_builds() {
        let vectors = legacy();
        let expected = &vectors["login"];
        let request = login();
        let responder = decode_i_address(expected["responder"].as_str().unwrap()).unwrap();
        let decision = request
            .decision(expected["responseCreatedAt"].as_u64().unwrap())
            .unwrap();
        let signature = hex::decode(vectors["signature"].as_str().unwrap()).unwrap();
        assert_eq!(
            hex::encode(
                login_response(request.system_id, responder, &signature, &decision).unwrap()
            ),
            expected["responseHex"].as_str().unwrap()
        );
        assert_eq!(
            hex::encode(decision_hash(
                request.system_id,
                responder,
                vectors["height"].as_u64().unwrap() as u32,
                &decision
            )),
            expected["decisionHash"].as_str().unwrap()
        );
    }

    #[test]
    fn a_webhook_body_must_repeat_the_signed_response() {
        use base64::engine::general_purpose::STANDARD;
        let vectors = legacy();
        let expected = &vectors["login"];
        let response =
            parse_login_response(&hex::decode(expected["responseHex"].as_str().unwrap()).unwrap())
                .unwrap();
        let signature = hex::decode(vectors["signature"].as_str().unwrap()).unwrap();
        let mut body = serde_json::json!({
            "system_id": "iJhCezBExJHvtyH3fGhNnt2NhU4Ztkf2yq",
            "signing_id": expected["responder"],
            "signature": { "signature": STANDARD.encode(&signature) },
            "decision": {
                "decision_id": expected["challengeId"],
                "created_at": expected["responseCreatedAt"],
            },
        });
        assert!(check_webhook_body(&response, &body).is_ok());
        body["decision"]["created_at"] = serde_json::json!(1);
        assert!(check_webhook_body(&response, &body).is_err());
    }

    #[test]
    fn logins_with_context_or_other_permissions_are_refused() {
        let vectors = legacy();
        let parse = |name: &str| match parse_legacy_link(vectors[name].as_str().unwrap()).unwrap() {
            LegacyLink::Login(bytes) => parse_login_consent(&bytes).unwrap(),
            other => panic!("expected a login link, got {other:?}"),
        };
        assert_eq!(
            check_login_consent(&parse("loginWithContext"), true),
            Err(Rejection::LoginContext)
        );
        assert_eq!(
            check_login_consent(&parse("loginOtherPermission"), true),
            Err(Rejection::LoginPermission)
        );
    }

    #[test]
    fn legacy_invoices_become_version_4_details() {
        let vectors = legacy();
        let height = vectors["height"].as_u64().unwrap() as u32;
        for case in vectors["invoices"].as_array().unwrap() {
            let invoice = match parse_legacy_link(case["link"].as_str().unwrap()).unwrap() {
                LegacyLink::Invoice(payload) => parse_legacy_invoice(&payload).unwrap(),
                other => panic!("expected an invoice link, got {other:?}"),
            };
            let name = format!("v{} signed={}", case["version"], case["signed"]);
            assert_eq!(
                hex::encode(&invoice.details_v4),
                case["detailsV4"].as_str().unwrap(),
                "{name}"
            );
            assert_eq!(invoice.signer.is_some(), case["signed"].as_bool().unwrap());
            assert_eq!(
                hex::encode(invoice.details_hash(height, 2)),
                case["detailsHash"].as_str().unwrap(),
                "{name}"
            );
        }
    }

    #[test]
    fn verus_links_and_other_hosts_are_not_legacy() {
        let link = legacy()["login"]["link"].as_str().unwrap().to_string();
        assert!(is_legacy_link(&link));
        assert!(!is_legacy_link(&link.replacen(
            "i5jtwbp6zymeay9llnraglgjqgdrffsau4",
            "verus",
            1
        )));
        assert!(parse_legacy_link(&link.replace("x-callback-url", "example.com")).is_err());
    }
}
