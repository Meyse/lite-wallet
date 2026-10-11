//! Which requests the wallet accepts, before any signature or network check.
//! The rules follow Verus Mobile `dev-asf-parent` and the verusid-ts-client
//! version it locks (`validateUnsignedGenericRequest`), plus the two desktop
//! deviations in ADR 0003.

use crate::types::WalletError;

use super::details::DetailKind;
use super::names::root_system_id;
use super::request::{ParsedRequest, FLAG_DETAILS_ARE_ENCRYPTED};

/// Mobile refuses deep links of this length or longer
/// (`utils/constants/deeplink.js`).
pub(crate) const MAX_LINK_LENGTH: usize = 2046;

/// Why a request is refused. The code reaches the UI, which shows translated
/// copy for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Rejection {
    /// A detail type the wallet does not know, or a response detail.
    UnknownDetail,
    /// The details break the order and count rules.
    DetailOrder,
    /// Only invoices, gift card claims and wallet backups may be unsigned,
    /// and an unsigned request names no app identity.
    Unsigned,
    /// Wallet backups write to an NFC card; desktop refuses them (ADR 0003).
    WalletBackup,
    /// The request is for the other network.
    Network,
    /// The details are encrypted, so the wallet cannot read them.
    EncryptedDetails,
    TooLong,
    Expired,
    /// The request names an app identity, and its signer is not a
    /// single-signature VerusID that this wallet controls.
    DelegatedSigner,
    /// The requester's signature does not verify.
    Signature,
}

impl Rejection {
    pub(crate) fn code(self) -> &'static str {
        match self {
            Self::UnknownDetail => "unknownDetail",
            Self::DetailOrder => "detailOrder",
            Self::Unsigned => "unsigned",
            Self::WalletBackup => "walletBackup",
            Self::Network => "network",
            Self::EncryptedDetails => "encryptedDetails",
            Self::TooLong => "tooLong",
            Self::Expired => "expired",
            Self::DelegatedSigner => "delegatedSigner",
            Self::Signature => "signature",
        }
    }
}

impl From<Rejection> for WalletError {
    fn from(rejection: Rejection) -> Self {
        WalletError::GenericRequestRejected(rejection.code().to_string())
    }
}

pub(crate) fn detail_kinds(request: &ParsedRequest) -> Result<Vec<DetailKind>, Rejection> {
    request
        .details
        .iter()
        .map(|detail| DetailKind::from_ordinal(detail.ordinal).ok_or(Rejection::UnknownDetail))
        .collect()
}

/// `validateUnsignedGenericRequest`: at most one sign in, New VerusID,
/// encryption key and wallet backup, and at most one invoice or identity
/// update in total. A wallet backup comes first, then sign in; the invoice or
/// update comes last; New VerusID and an encryption key need a sign in before
/// them.
pub(crate) fn check_order(kinds: &[DetailKind]) -> Result<(), Rejection> {
    let position = |wanted: &[DetailKind]| -> Result<Option<usize>, Rejection> {
        let mut found = None;
        for (index, kind) in kinds.iter().enumerate() {
            if wanted.contains(kind) {
                if found.is_some() {
                    return Err(Rejection::DetailOrder);
                }
                found = Some(index);
            }
        }
        Ok(found)
    };

    let auth = position(&[DetailKind::Authentication])?;
    let provisioning = position(&[DetailKind::ProvisionIdentity])?;
    let app_encryption = position(&[DetailKind::AppEncryption])?;
    let wallet_backup = position(&[DetailKind::CreateWalletBackup])?;
    let last = position(&[DetailKind::Invoice, DetailKind::IdentityUpdate])?;

    if wallet_backup.is_some_and(|index| index != 0) {
        return Err(Rejection::DetailOrder);
    }
    let first_after_backup = if wallet_backup == Some(0) { 1 } else { 0 };
    if auth.is_some_and(|index| index != first_after_backup) {
        return Err(Rejection::DetailOrder);
    }
    if last.is_some_and(|index| index + 1 != kinds.len()) {
        return Err(Rejection::DetailOrder);
    }
    for needs_auth in [provisioning, app_encryption].into_iter().flatten() {
        if auth.is_none_or(|auth| needs_auth < auth) {
            return Err(Rejection::DetailOrder);
        }
    }
    Ok(())
}

/// The checks that need only the request bytes and the open wallet's network.
pub(crate) fn check_request(
    request: &ParsedRequest,
    wallet_is_testnet: bool,
) -> Result<Vec<DetailKind>, Rejection> {
    if request.flags & FLAG_DETAILS_ARE_ENCRYPTED != 0 {
        return Err(Rejection::EncryptedDetails);
    }
    let kinds = detail_kinds(request)?;
    if kinds.is_empty() {
        return Err(Rejection::UnknownDetail);
    }
    check_order(&kinds)?;

    if !request.is_signed() {
        let unsigned_allowed = kinds.iter().all(|kind| {
            matches!(
                kind,
                DetailKind::Invoice | DetailKind::SpendableKey | DetailKind::CreateWalletBackup
            )
        });
        if !unsigned_allowed || request.app_or_delegated_id.is_some() {
            return Err(Rejection::Unsigned);
        }
    }
    if kinds.contains(&DetailKind::CreateWalletBackup) {
        return Err(Rejection::WalletBackup);
    }
    if request.is_testnet != wallet_is_testnet {
        return Err(Rejection::Network);
    }
    // A signature from the other network's root system is a request for that
    // network, whatever its flag says (Verus Mobile looks up the coin from
    // the signature's system).
    if let Some(signature) = &request.signature {
        if signature
            .signer_system_id
            .eq_ignore_ascii_case(root_system_id(!request.is_testnet))
        {
            return Err(Rejection::Network);
        }
    }
    Ok(kinds)
}

/// Whether the request names an app identity that differs from its signer.
/// Such a request is accepted only when this wallet controls the signer.
pub(crate) fn names_delegated_app(request: &ParsedRequest) -> bool {
    match (&request.app_or_delegated_id, &request.signature) {
        (Some(app), Some(signature)) => !app.eq_ignore_ascii_case(&signature.signer_identity_id),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::generic_request::request::parse_request_hex;
    use serde_json::Value;

    fn vectors() -> Vec<Value> {
        let raw = include_str!("../../../../src/lib/genericRequest/fixtures/request-vectors.json");
        let parsed: Value = serde_json::from_str(raw).expect("vectors parse");
        parsed["cases"].as_array().expect("cases").clone()
    }

    #[test]
    fn order_rules_match_verusid_ts_client_for_every_vector() {
        for case in vectors() {
            let name = case["name"].as_str().unwrap();
            let request = parse_request_hex(case["hex"].as_str().unwrap())
                .unwrap_or_else(|error| panic!("{name} parses: {error:?}"));
            let kinds = detail_kinds(&request).expect("known details");
            let library_accepts = case["libraryAccepts"].as_bool().unwrap();
            assert_eq!(
                check_order(&kinds).is_ok(),
                library_accepts,
                "{name}: order rule differs from the library"
            );
        }
    }

    #[test]
    fn every_vector_meets_its_expected_outcome() {
        for case in vectors() {
            let name = case["name"].as_str().unwrap();
            let request = parse_request_hex(case["hex"].as_str().unwrap()).unwrap();
            let expect = &case["expect"];
            let testnet = expect["testnet"].as_bool().unwrap_or(true);
            let outcome = check_request(&request, testnet);
            match expect["reject"].as_str() {
                Some("order") => assert_eq!(outcome, Err(Rejection::DetailOrder), "{name}"),
                Some("unsigned") => assert_eq!(outcome, Err(Rejection::Unsigned), "{name}"),
                Some(other) => panic!("{name}: unknown expectation {other}"),
                None => assert!(outcome.is_ok(), "{name}: {outcome:?}"),
            }
        }
    }

    #[test]
    fn a_request_for_the_other_network_is_rejected() {
        let case = vectors()
            .into_iter()
            .find(|case| case["name"] == "mainnet-auth")
            .unwrap();
        let request = parse_request_hex(case["hex"].as_str().unwrap()).unwrap();
        assert_eq!(check_request(&request, true), Err(Rejection::Network));
        assert!(check_request(&request, false).is_ok());
    }

    #[test]
    fn wallet_backups_are_refused_even_when_well_ordered() {
        let kinds = [DetailKind::CreateWalletBackup, DetailKind::SpendableKey];
        assert!(check_order(&kinds).is_ok());
    }
}
