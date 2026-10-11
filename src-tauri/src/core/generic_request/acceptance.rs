//! Which requests the wallet accepts, before any signature or network check.
//! The rules follow Verus Mobile `dev-asf-parent` and the verusid-ts-client
//! version it locks (`validateUnsignedGenericRequest`), plus the two desktop
//! deviations in ADR 0003.

use crate::types::WalletError;

use super::details::DetailKind;
use super::names::root_system_id;
use super::request::{ParsedRequest, FLAG_DETAILS_ARE_ENCRYPTED};

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
    Expired,
    /// The request names an app identity, and its signer is not a
    /// single-signature VerusID that this wallet controls.
    DelegatedSigner,
    /// The requester's signature does not verify.
    Signature,
    /// A detail type that desktop cannot answer yet.
    UnsupportedDetail,
    InvoiceTagged,
    /// The invoice can be paid only outside the Verus chain.
    InvoiceUnpayable,
    /// The invoice pays a private address.
    InvoicePrivate,
    /// The invoice names no address the wallet can pay.
    InvoiceNoDestination,
    /// The invoice's currency is unknown on this network.
    InvoiceCurrency,
    /// The invoice preconverts to a currency that has launched.
    InvoicePreconvertLaunched,
    /// This invoice was already paid in this wallet session.
    AlreadyPaid,
    /// The data packet sends data to the user instead of asking for a
    /// signature; Verus Mobile refuses it too.
    DataTransmittal,
    /// The data packet does not ask for the user's signature, or names
    /// nothing to sign.
    DataNotForSignature,
    /// The app asks for a spending key without naming an address to encrypt
    /// it to, or names an address that is not valid.
    EncryptionKeyAddress,
    /// The derivation identity of an encryption key request does not exist.
    EncryptionKeyIdentity,
    /// This wallet has no private address set up.
    PrivateNotSetUp,
    /// The app asks for data other than whole credentials; Verus Mobile
    /// answers only full credential requests.
    CredentialsOnly,
    /// A credential request names no address to encrypt the answer to.
    CredentialsNotEncrypted,
    /// An update writes credentials to an identity whose private address
    /// does not come from this wallet's seed.
    CredentialAddress,
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
            Self::Expired => "expired",
            Self::DelegatedSigner => "delegatedSigner",
            Self::Signature => "signature",
            Self::UnsupportedDetail => "unsupportedDetail",
            Self::InvoiceTagged => "invoiceTagged",
            Self::InvoiceUnpayable => "invoiceUnpayable",
            Self::InvoicePrivate => "invoicePrivate",
            Self::InvoiceNoDestination => "invoiceNoDestination",
            Self::InvoiceCurrency => "invoiceCurrency",
            Self::InvoicePreconvertLaunched => "invoicePreconvertLaunched",
            Self::AlreadyPaid => "alreadyPaid",
            Self::DataTransmittal => "dataTransmittal",
            Self::DataNotForSignature => "dataNotForSignature",
            Self::EncryptionKeyAddress => "encryptionKeyAddress",
            Self::EncryptionKeyIdentity => "encryptionKeyIdentity",
            Self::PrivateNotSetUp => "privateNotSetUp",
            Self::CredentialsOnly => "credentialsOnly",
            Self::CredentialsNotEncrypted => "credentialsNotEncrypted",
            Self::CredentialAddress => "credentialAddress",
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

/// `dataPacketRequestValidator.js`: a data packet asks for the user's
/// signature over at least one object, and never sends data to the user.
pub(crate) fn check_data_packet(
    packet: &super::details::DataPacketRequest,
) -> Result<(), Rejection> {
    use super::details::*;
    if packet.flags & DATA_PACKET_FLAG_FOR_TRANSMITTAL_TO_USER != 0 {
        return Err(Rejection::DataTransmittal);
    }
    if packet.flags & DATA_PACKET_FLAG_FOR_USERS_SIGNATURE == 0 || packet.object_count == 0 {
        return Err(Rejection::DataNotForSignature);
    }
    Ok(())
}

/// `appEncryptionRequestValidator.js`: a spending key is returned only
/// encrypted, to a valid address named in the detail. The derivation
/// identity is checked against the chain when the request opens.
pub(crate) fn check_app_encryption(
    request: &super::details::AppEncryptionRequest,
) -> Result<(), Rejection> {
    if let Some(raw) = &request.encrypt_to {
        if sapling::PaymentAddress::from_bytes(raw).is_none() {
            return Err(Rejection::EncryptionKeyAddress);
        }
    } else if request.return_spending_key {
        return Err(Rejection::EncryptionKeyAddress);
    }
    Ok(())
}

/// `userDataRequestValidator.js`: a valid request for full credentials, with
/// an address to encrypt the answer to. The signed-request rule is checked
/// with the other unsigned rules.
pub(crate) fn check_user_data(
    request: &ParsedRequest,
    user_data: &super::details::UserDataRequest,
) -> Result<(), Rejection> {
    use super::details::{USER_DATA_CREDENTIAL, USER_DATA_FULL_DATA};
    if !user_data.is_valid()
        || user_data.request_type != USER_DATA_CREDENTIAL
        || user_data.data_type != USER_DATA_FULL_DATA
    {
        return Err(Rejection::CredentialsOnly);
    }
    if request.encrypt_response_to.is_none() {
        return Err(Rejection::CredentialsNotEncrypted);
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
    for (detail, kind) in request.details.iter().zip(&kinds) {
        if *kind == DetailKind::DataPacket {
            let packet = super::details::parse_data_packet(&detail.data, request.is_testnet)
                .map_err(|_| Rejection::DataNotForSignature)?;
            check_data_packet(&packet)?;
        }
        if *kind == DetailKind::AppEncryption {
            let app = super::details::parse_app_encryption(&detail.data, request.is_testnet)
                .map_err(|_| Rejection::EncryptionKeyAddress)?;
            check_app_encryption(&app)?;
        }
        if *kind == DetailKind::UserData {
            let user_data = super::details::parse_user_data(&detail.data, request.is_testnet)
                .map_err(|_| Rejection::CredentialsOnly)?;
            check_user_data(request, &user_data)?;
        }
        if *kind == DetailKind::IdentityUpdate {
            // Sign data in an update is signed by the daemon on Verus Mobile;
            // this wallet builds updates itself and cannot sign it yet.
            let update = super::details::parse_identity_update(&detail.data, request.is_testnet)
                .map_err(|_| Rejection::UnsupportedDetail)?;
            if update.contains_sign_data {
                return Err(Rejection::UnsupportedDetail);
            }
        }
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
    if let Some(raw) = &request.encrypt_response_to {
        if sapling::PaymentAddress::from_bytes(raw).is_none() {
            return Err(Rejection::EncryptionKeyAddress);
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
                Some(code) => assert_eq!(
                    outcome.map(|_| ()).map_err(Rejection::code),
                    Err(code),
                    "{name}"
                ),
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

/// The invoice rules that need only the invoice: what the desktop wallet can
/// pay. Expiry, the currency, and the preconvert window need the chain.
pub(crate) fn check_invoice(
    invoice: &super::details::InvoiceRequest,
    request_is_testnet: bool,
) -> Result<(), Rejection> {
    use super::details::*;
    if invoice.has(INVOICE_IS_TESTNET) != request_is_testnet {
        return Err(Rejection::Network);
    }
    if invoice.has(INVOICE_IS_TAGGED) {
        return Err(Rejection::InvoiceTagged);
    }
    // The desktop wallet pays on the Verus chain only.
    if invoice.has(INVOICE_EXCLUDES_VERUS_BLOCKCHAIN) {
        return Err(Rejection::InvoiceUnpayable);
    }
    // Burning to change a currency's price and preconverting to a currency
    // before launch arrive with conversion.
    if invoice.has(INVOICE_IS_BURN_CHANGE_PRICE) {
        return Err(Rejection::UnsupportedDetail);
    }
    match &invoice.destination {
        Some(InvoiceDestination::Transparent(_)) => Ok(()),
        Some(InvoiceDestination::Private(_)) => Err(Rejection::InvoicePrivate),
        Some(InvoiceDestination::Unsupported) | None => Err(Rejection::InvoiceNoDestination),
    }
}

#[cfg(test)]
mod invoice_tests {
    use super::*;
    use crate::core::generic_request::details::{
        parse_invoice, InvoiceDestination, INVOICE_IS_TAGGED, INVOICE_IS_TESTNET,
    };
    use crate::core::generic_request::request::parse_request_hex;
    use serde_json::Value;

    fn invoice_vector() -> (super::super::details::InvoiceRequest, Value) {
        let raw = include_str!("../../../../src/lib/genericRequest/fixtures/request-vectors.json");
        let parsed: Value = serde_json::from_str(raw).unwrap();
        let case = parsed["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|case| case["name"] == "invoice-unsigned")
            .unwrap()
            .clone();
        let request = parse_request_hex(case["hex"].as_str().unwrap()).unwrap();
        (
            parse_invoice(&request.details[0].data).unwrap(),
            case["expect"]["invoice"].clone(),
        )
    }

    #[test]
    fn invoice_fields_match_the_library() {
        let (invoice, expected) = invoice_vector();
        assert_eq!(invoice.amount, expected["amount"].as_u64());
        assert_eq!(
            invoice.destination,
            Some(InvoiceDestination::Transparent(
                expected["destination"].as_str().unwrap().to_string()
            ))
        );
        assert_eq!(
            invoice.requested_currency_id,
            expected["currency"].as_str().unwrap()
        );
        assert_eq!(invoice.expiry_height, expected["expiryHeight"].as_u64());
        assert!(check_invoice(&invoice, true).is_ok());
    }

    #[test]
    fn invoices_desktop_cannot_pay_are_refused() {
        let (invoice, _) = invoice_vector();
        assert_eq!(check_invoice(&invoice, false), Err(Rejection::Network));

        let mut tagged = invoice.clone();
        tagged.flags |= INVOICE_IS_TAGGED;
        assert_eq!(check_invoice(&tagged, true), Err(Rejection::InvoiceTagged));

        let mut private = invoice.clone();
        private.destination = Some(InvoiceDestination::Private("zs1".to_string()));
        assert_eq!(
            check_invoice(&private, true),
            Err(Rejection::InvoicePrivate)
        );

        let mut anywhere = invoice.clone();
        anywhere.destination = None;
        assert_eq!(
            check_invoice(&anywhere, true),
            Err(Rejection::InvoiceNoDestination)
        );

        let mut mainnet = invoice;
        mainnet.flags &= !INVOICE_IS_TESTNET;
        assert_eq!(check_invoice(&mainnet, true), Err(Rejection::Network));
    }
}
