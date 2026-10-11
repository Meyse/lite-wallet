//! Parsers for request details. Each reads exactly the detail's payload and
//! returns the values the wallet acts on (ADR 0002).

use crate::types::WalletError;

use super::names::{
    base58_check, decode_i_address, i_address, name_id, root_system_name, sapling_address,
    R_ADDRESS_VERSION,
};
use super::request::CompactAddress;
use super::wire::{invalid, Reader};

pub(crate) const ORDINAL_INVOICE: u64 = 1;
pub(crate) const ORDINAL_AUTHENTICATION: u64 = 2;
pub(crate) const ORDINAL_IDENTITY_UPDATE: u64 = 4;
pub(crate) const ORDINAL_PROVISION_IDENTITY: u64 = 6;
pub(crate) const ORDINAL_APP_ENCRYPTION: u64 = 7;
pub(crate) const ORDINAL_DATA_PACKET: u64 = 9;
pub(crate) const ORDINAL_USER_DATA: u64 = 10;
pub(crate) const ORDINAL_CREATE_WALLET_BACKUP: u64 = 12;
pub(crate) const ORDINAL_SPENDABLE_KEY: u64 = 14;

/// The request detail types. Response ordinals (3, 5, 8, 11, 13) and the data
/// descriptor (0) never appear in a request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DetailKind {
    Invoice,
    Authentication,
    IdentityUpdate,
    ProvisionIdentity,
    AppEncryption,
    DataPacket,
    UserData,
    CreateWalletBackup,
    SpendableKey,
}

impl DetailKind {
    pub(crate) fn from_ordinal(ordinal: u64) -> Option<Self> {
        Some(match ordinal {
            ORDINAL_INVOICE => Self::Invoice,
            ORDINAL_AUTHENTICATION => Self::Authentication,
            ORDINAL_IDENTITY_UPDATE => Self::IdentityUpdate,
            ORDINAL_PROVISION_IDENTITY => Self::ProvisionIdentity,
            ORDINAL_APP_ENCRYPTION => Self::AppEncryption,
            ORDINAL_DATA_PACKET => Self::DataPacket,
            ORDINAL_USER_DATA => Self::UserData,
            ORDINAL_CREATE_WALLET_BACKUP => Self::CreateWalletBackup,
            ORDINAL_SPENDABLE_KEY => Self::SpendableKey,
            _ => return None,
        })
    }
}

// --- Sign in -----------------------------------------------------------------

const AUTH_FLAG_HAS_REQUEST_ID: u64 = 1;
const AUTH_FLAG_HAS_RECIPIENT_CONSTRAINTS: u64 = 2;
const AUTH_FLAG_HAS_EXPIRY_TIME: u64 = 4;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RecipientConstraint {
    /// 1 required ID, 2 required system, 3 required parent.
    pub kind: u64,
    pub identity: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AuthenticationRequest {
    pub request_id: Option<String>,
    pub constraints: Vec<RecipientConstraint>,
    pub expiry_time: Option<u64>,
}

pub(crate) fn parse_authentication(
    data: &[u8],
    is_testnet: bool,
) -> Result<AuthenticationRequest, WalletError> {
    let mut reader = Reader::new(data);
    let flags = reader.compact_size()?;
    let request_id = if flags & AUTH_FLAG_HAS_REQUEST_ID != 0 {
        Some(CompactAddress::read(&mut reader)?.identity_address(is_testnet)?)
    } else {
        None
    };
    let mut constraints = Vec::new();
    if flags & AUTH_FLAG_HAS_RECIPIENT_CONSTRAINTS != 0 {
        let count = reader.count()?;
        for _ in 0..count {
            let kind = reader.compact_size()?;
            let identity = CompactAddress::read(&mut reader)?.identity_address(is_testnet)?;
            constraints.push(RecipientConstraint { kind, identity });
        }
    }
    let expiry_time = if flags & AUTH_FLAG_HAS_EXPIRY_TIME != 0 {
        Some(reader.compact_size()?)
    } else {
        None
    };
    reader.finish()?;
    Ok(AuthenticationRequest {
        request_id,
        constraints,
        expiry_time,
    })
}

// --- Update identity ---------------------------------------------------------

const UPDATE_FLAG_CONTAINS_SIGNDATA: u64 = 1;
const UPDATE_FLAG_EXPIRES: u64 = 2;
const UPDATE_FLAG_CONTAINS_REQUEST_ID: u64 = 4;
const UPDATE_FLAG_CONTAINS_SYSTEM: u64 = 8;
const UPDATE_FLAG_CONTAINS_TXID: u64 = 16;

const PARTIAL_CONTAINS_PARENT: u64 = 1;
const PARTIAL_CONTAINS_CONTENT_MULTIMAP: u64 = 2;
const PARTIAL_CONTAINS_PRIMARY_ADDRS: u64 = 4;
const PARTIAL_CONTAINS_REVOCATION: u64 = 8;
const PARTIAL_CONTAINS_RECOVERY: u64 = 16;
const PARTIAL_CONTAINS_UNLOCK_AFTER: u64 = 32;
const PARTIAL_CONTAINS_SYSTEM_ID: u64 = 64;
const PARTIAL_CONTAINS_PRIV_ADDRS: u64 = 128;
const PARTIAL_CONTAINS_CONTENT_MAP: u64 = 256;
const PARTIAL_CONTAINS_MINSIGS: u64 = 512;
const PARTIAL_CONTAINS_FLAGS: u64 = 1024;
const PARTIAL_CONTAINS_VERSION: u64 = 2048;

const IDENTITY_VERSION_VAULT: u32 = 2;
const IDENTITY_VERSION_PBAAS: u32 = 3;

/// The identity fields an update request sets, as `PartialIdentity` holds them.
/// Content values stay raw: they change no control of the identity.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct PartialIdentity {
    pub version: Option<u32>,
    pub flags: Option<u32>,
    pub primary_addresses: Option<Vec<String>>,
    pub minimum_signatures: Option<u32>,
    pub parent: Option<String>,
    pub name: String,
    /// Each key as the request names it, with its values in order.
    pub content_multimap: Option<Vec<(String, Vec<Vec<u8>>)>>,
    pub content_map: Option<Vec<(String, [u8; 32])>>,
    pub revocation: Option<String>,
    pub recovery: Option<String>,
    pub private_addresses: Option<Vec<String>>,
    pub system_id: Option<String>,
    pub unlock_after: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct IdentityUpdateRequest {
    pub request_id: Option<String>,
    pub identity: PartialIdentity,
    pub expiry_height: Option<u64>,
    pub system_id: Option<String>,
    /// Display-order txid of the identity transaction the request was made for.
    pub txid: Option<String>,
    pub contains_sign_data: bool,
}

/// A content key as the request writes it: an i-address, or a name such as
/// `ns@::key.name` that the daemon resolves.
fn content_key_text(key: &CompactAddress) -> Result<String, WalletError> {
    match key {
        CompactAddress::IAddress(hash) => Ok(i_address(hash)),
        CompactAddress::Name(name) => Ok(name.clone()),
        CompactAddress::XAddress(_) => Err(invalid()),
    }
}

fn parse_partial_identity(reader: &mut Reader<'_>) -> Result<PartialIdentity, WalletError> {
    let contains = reader.varint()?;
    let mut identity = PartialIdentity::default();

    // Principal
    if contains & PARTIAL_CONTAINS_VERSION != 0 {
        identity.version = Some(reader.u32_le()?);
    }
    if contains & PARTIAL_CONTAINS_FLAGS != 0 {
        identity.flags = Some(reader.u32_le()?);
    }
    if contains & PARTIAL_CONTAINS_PRIMARY_ADDRS != 0 {
        let count = reader.count()?;
        let mut addresses = Vec::with_capacity(count);
        for _ in 0..count {
            let key = reader.var_slice()?;
            let hash: [u8; 20] = key.try_into().map_err(|_| invalid())?;
            addresses.push(base58_check(R_ADDRESS_VERSION, &hash));
        }
        identity.primary_addresses = Some(addresses);
    }
    if contains & PARTIAL_CONTAINS_MINSIGS != 0 {
        identity.minimum_signatures = Some(reader.u32_le()?);
    }

    // Identity. A partial identity without a version is read as the current one.
    let version = identity.version.unwrap_or(IDENTITY_VERSION_PBAAS);
    if contains & PARTIAL_CONTAINS_PARENT != 0 {
        identity.parent = Some(i_address(&reader.hash160()?));
    }
    identity.name = reader.utf8()?;
    if contains & PARTIAL_CONTAINS_CONTENT_MULTIMAP != 0 && version >= IDENTITY_VERSION_PBAAS {
        let count = reader.count()?;
        let mut entries = Vec::with_capacity(count);
        for _ in 0..count {
            let key = content_key_text(&CompactAddress::read(reader)?)?;
            let value_count = reader.count()?;
            let mut values = Vec::with_capacity(value_count);
            for _ in 0..value_count {
                values.push(reader.var_slice()?.to_vec());
            }
            entries.push((key, values));
        }
        identity.content_multimap = Some(entries);
    }
    if contains & PARTIAL_CONTAINS_CONTENT_MAP != 0 {
        let count = reader.count()?;
        let mut entries = Vec::with_capacity(count);
        for _ in 0..count {
            let key = i_address(&reader.hash160()?);
            entries.push((key, reader.hash256()?));
        }
        identity.content_map = Some(entries);
    }
    if contains & PARTIAL_CONTAINS_REVOCATION != 0 {
        identity.revocation = Some(i_address(&reader.hash160()?));
    }
    if contains & PARTIAL_CONTAINS_RECOVERY != 0 {
        identity.recovery = Some(i_address(&reader.hash160()?));
    }
    if contains & PARTIAL_CONTAINS_PRIV_ADDRS != 0 {
        let count = reader.count()?;
        let mut addresses = Vec::with_capacity(count);
        for _ in 0..count {
            let raw: [u8; 43] = reader.take(43)?.try_into().map_err(|_| invalid())?;
            addresses.push(sapling_address(&raw).ok_or_else(invalid)?);
        }
        identity.private_addresses = Some(addresses);
    }
    if version >= IDENTITY_VERSION_VAULT {
        if contains & PARTIAL_CONTAINS_SYSTEM_ID != 0 {
            identity.system_id = Some(i_address(&reader.hash160()?));
        }
        if contains & PARTIAL_CONTAINS_UNLOCK_AFTER != 0 {
            identity.unlock_after = Some(reader.u32_le()?);
        }
    }

    Ok(identity)
}

pub(crate) fn parse_identity_update(
    data: &[u8],
    is_testnet: bool,
) -> Result<IdentityUpdateRequest, WalletError> {
    let mut reader = Reader::new(data);
    let flags = reader.compact_size()?;
    let request_id = if flags & UPDATE_FLAG_CONTAINS_REQUEST_ID != 0 {
        Some(CompactAddress::read(&mut reader)?.identity_address(is_testnet)?)
    } else {
        None
    };
    let identity = parse_partial_identity(&mut reader)?;
    let expiry_height = if flags & UPDATE_FLAG_EXPIRES != 0 {
        Some(reader.compact_size()?)
    } else {
        None
    };
    let system_id = if flags & UPDATE_FLAG_CONTAINS_SYSTEM != 0 {
        Some(i_address(&reader.hash160()?))
    } else {
        None
    };
    let txid = if flags & UPDATE_FLAG_CONTAINS_TXID != 0 {
        let mut txid = reader.hash256()?;
        txid.reverse();
        Some(hex::encode(txid))
    } else {
        None
    };
    let contains_sign_data = flags & UPDATE_FLAG_CONTAINS_SIGNDATA != 0;
    // Sign data (credentials written by the update) is read in phase 4; until
    // then the remaining bytes belong to it and nothing else may follow.
    if !contains_sign_data {
        reader.finish()?;
    }
    Ok(IdentityUpdateRequest {
        request_id,
        identity,
        expiry_height,
        system_id,
        txid,
        contains_sign_data,
    })
}

impl IdentityUpdateRequest {
    /// The identity the update targets, as `getIdentityAddress(isTestnet)`
    /// derives it: the name under its parent, or under the root system when the
    /// request gives no parent. The root systems hash with no parent.
    pub(crate) fn target_identity(&self, is_testnet: bool) -> Option<String> {
        let name = self.identity.name.as_str();
        if name.is_empty() {
            return None;
        }
        if name == "VRSC" || name == "VRSCTEST" {
            return Some(i_address(&name_id(name, None)));
        }
        let parent = match self.identity.parent.as_deref() {
            Some(parent) => decode_i_address(parent)?,
            None => name_id(root_system_name(is_testnet), None),
        };
        Some(i_address(&name_id(name, Some(&parent))))
    }
}

// --- New VerusID -------------------------------------------------------------

const PROVISION_FLAG_HAS_SYSTEM_ID: u64 = 1;
const PROVISION_FLAG_HAS_PARENT_ID: u64 = 2;
const PROVISION_FLAG_HAS_IDENTITY_ID: u64 = 4;
const PROVISION_FLAG_HAS_URI: u64 = 8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProvisionIdentityRequest {
    pub uri: Option<String>,
    pub system_id: Option<String>,
    pub parent_id: Option<String>,
    /// The offered identity, as the request names it.
    pub identity: Option<String>,
}

pub(crate) fn parse_provision_identity(
    data: &[u8],
    is_testnet: bool,
) -> Result<ProvisionIdentityRequest, WalletError> {
    let mut reader = Reader::new(data);
    let flags = reader.compact_size()?;
    let uri = if flags & PROVISION_FLAG_HAS_URI != 0 {
        let _kind = reader.compact_size()?;
        Some(reader.utf8()?)
    } else {
        None
    };
    let system_id = if flags & PROVISION_FLAG_HAS_SYSTEM_ID != 0 {
        Some(CompactAddress::read(&mut reader)?.identity_address(is_testnet)?)
    } else {
        None
    };
    let parent_id = if flags & PROVISION_FLAG_HAS_PARENT_ID != 0 {
        Some(CompactAddress::read(&mut reader)?.identity_address(is_testnet)?)
    } else {
        None
    };
    let identity = if flags & PROVISION_FLAG_HAS_IDENTITY_ID != 0 {
        Some(match CompactAddress::read(&mut reader)? {
            CompactAddress::Name(name) => name,
            other => other.identity_address(is_testnet)?,
        })
    } else {
        None
    };
    reader.finish()?;
    Ok(ProvisionIdentityRequest {
        uri,
        system_id,
        parent_id,
        identity,
    })
}

/// The identity JSON for `updateidentity`, built from the verified request.
/// Every control field comes from the request bytes. Content values are taken
/// from `ui_json` (the library's `toCLIJson()`), but only under the keys the
/// request names and with the same number of values: content changes no
/// control of the identity, and Rust has no VDXF object decoder.
pub(crate) fn requested_identity_json(
    update: &IdentityUpdateRequest,
    ui_json: &serde_json::Value,
) -> Result<serde_json::Value, WalletError> {
    use serde_json::{json, Map, Value};

    let identity = &update.identity;
    let mut out = Map::new();
    out.insert("name".into(), json!(identity.name));
    if let Some(version) = identity.version {
        out.insert("version".into(), json!(version));
    }
    if let Some(flags) = identity.flags {
        out.insert("flags".into(), json!(flags));
    }
    if let Some(addresses) = &identity.primary_addresses {
        out.insert("primaryaddresses".into(), json!(addresses));
    }
    if let Some(minimum) = identity.minimum_signatures {
        out.insert("minimumsignatures".into(), json!(minimum));
    }
    if let Some(parent) = &identity.parent {
        out.insert("parent".into(), json!(parent));
    }
    if let Some(revocation) = &identity.revocation {
        out.insert("revocationauthority".into(), json!(revocation));
    }
    if let Some(recovery) = &identity.recovery {
        out.insert("recoveryauthority".into(), json!(recovery));
    }
    if let Some(address) = identity
        .private_addresses
        .as_ref()
        .and_then(|addresses| addresses.first())
    {
        out.insert("privateaddress".into(), json!(address));
    }
    if let Some(system_id) = &identity.system_id {
        out.insert("systemid".into(), json!(system_id));
    }
    if let Some(unlock_after) = identity.unlock_after {
        out.insert("timelock".into(), json!(unlock_after));
    }
    if let Some(entries) = &identity.content_map {
        // `Identity.toJson()` writes both key and value byte-reversed.
        let map = entries
            .iter()
            .map(|(key, value)| {
                let mut key_bytes = decode_i_address(key).ok_or_else(invalid)?.to_vec();
                key_bytes.reverse();
                let mut value_bytes = value.to_vec();
                value_bytes.reverse();
                Ok((hex::encode(key_bytes), json!(hex::encode(value_bytes))))
            })
            .collect::<Result<Map<String, Value>, WalletError>>()?;
        out.insert("contentmap".into(), Value::Object(map));
    }
    if let Some(entries) = &identity.content_multimap {
        let ui = ui_json
            .get("contentmultimap")
            .and_then(Value::as_object)
            .ok_or(WalletError::IdentityBuildFailed)?;
        let mut map = Map::new();
        for (key, values) in entries {
            // The daemon drops keys with no values.
            if values.is_empty() {
                continue;
            }
            let ui_values = match ui.get(key).ok_or(WalletError::IdentityBuildFailed)? {
                Value::Array(values) => values.clone(),
                value => vec![value.clone()],
            };
            if ui_values.len() != values.len() {
                return Err(WalletError::IdentityBuildFailed);
            }
            map.insert(key.clone(), Value::Array(ui_values));
        }
        let named_keys = entries
            .iter()
            .filter(|(_, values)| !values.is_empty())
            .count();
        let ui_keys = ui
            .iter()
            .filter(|(_, value)| value.as_array().is_none_or(|values| !values.is_empty()))
            .count();
        if ui_keys != named_keys {
            return Err(WalletError::IdentityBuildFailed);
        }
        out.insert("contentmultimap".into(), Value::Object(map));
    }
    Ok(Value::Object(out))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::generic_request::request::parse_request_hex;
    use serde_json::{json, Value};

    fn update_vector() -> IdentityUpdateRequest {
        let raw = include_str!("../../../../src/lib/genericRequest/fixtures/request-vectors.json");
        let parsed: Value = serde_json::from_str(raw).unwrap();
        let case = parsed["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|case| case["name"] == "auth-update")
            .unwrap()
            .clone();
        let request = parse_request_hex(case["hex"].as_str().unwrap()).unwrap();
        parse_identity_update(&request.details[1].data, true).unwrap()
    }

    fn ui_json(extra: Value) -> Value {
        let mut base = json!({
            "name": "player7",
            "contentmultimap": {
                "TestBob.antafri@::qa.note": [{ "iK7a5JNJnbeuYWVHCDRpJosj3irGJ5Qa8c": "Disposable QA note" }]
            }
        });
        base.as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        base
    }

    #[test]
    fn control_fields_come_from_the_request_not_the_ui() {
        let update = update_vector();
        // The UI asks for a different owner; Rust keeps the request's.
        let json = requested_identity_json(
            &update,
            &ui_json(json!({ "primaryaddresses": ["RAttacker1111111111111111111111111"] })),
        )
        .unwrap();
        assert_eq!(
            json["primaryaddresses"],
            json!(["RFWcHcpnFh57ovrV1hmzN8o8wmMZjyqCSh"])
        );
        assert_eq!(
            json["revocationauthority"],
            json!("i8KEYF3QeR1xPVVCL5BSpDXkA1CAfFDvmk")
        );
        assert_eq!(
            json["privateaddress"],
            json!("zs13akhfdhypx9a8t2efkfgxpd5d9h3glz7d5pug2ys65mrzt4mw5vjqx0vw29gx82y3e0xjjuhrwn")
        );
        assert_eq!(json["minimumsignatures"], json!(1));
    }

    #[test]
    fn content_values_must_sit_under_the_requested_keys() {
        let update = update_vector();
        assert!(requested_identity_json(&update, &ui_json(json!({}))).is_ok());
        let extra_key = json!({ "contentmultimap": {
            "TestBob.antafri@::qa.note": ["00"],
            "iK7a5JNJnbeuYWVHCDRpJosj3irGJ5Qa8c": ["00"]
        }});
        assert!(requested_identity_json(&update, &ui_json(extra_key)).is_err());
        let extra_value =
            json!({ "contentmultimap": { "TestBob.antafri@::qa.note": ["00", "01"] }});
        assert!(requested_identity_json(&update, &ui_json(extra_value)).is_err());
    }
}

// --- Pay invoice -------------------------------------------------------------

pub(crate) const INVOICE_ACCEPTS_CONVERSION: u64 = 2;
pub(crate) const INVOICE_ACCEPTS_NON_VERUS_SYSTEMS: u64 = 4;
pub(crate) const INVOICE_EXPIRES: u64 = 8;
pub(crate) const INVOICE_ACCEPTS_ANY_DESTINATION: u64 = 16;
pub(crate) const INVOICE_ACCEPTS_ANY_AMOUNT: u64 = 32;
pub(crate) const INVOICE_EXCLUDES_VERUS_BLOCKCHAIN: u64 = 64;
pub(crate) const INVOICE_IS_TESTNET: u64 = 128;
pub(crate) const INVOICE_IS_PRECONVERT: u64 = 256;
pub(crate) const INVOICE_DESTINATION_IS_SAPLING: u64 = 512;
pub(crate) const INVOICE_IS_TAGGED: u64 = 1024;
pub(crate) const INVOICE_IS_BURN_CHANGE_PRICE: u64 = 2048;

const DEST_PKH: u8 = 2;
const DEST_ID: u8 = 4;
const FLAG_DEST_AUX: u8 = 64;
const FLAG_DEST_GATEWAY: u8 = 128;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum InvoiceDestination {
    /// An R-address or a VerusID, as a transparent payment can reach it.
    Transparent(String),
    /// A private (Sapling) address.
    Private(String),
    /// A destination type the wallet does not pay to (gateways, keys, raw).
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InvoiceRequest {
    pub flags: u64,
    /// In satoshis. `None` when the invoice accepts any amount.
    pub amount: Option<u64>,
    /// `None` when the invoice accepts any destination.
    pub destination: Option<InvoiceDestination>,
    pub requested_currency_id: String,
    pub expiry_height: Option<u64>,
    pub accepted_systems: Vec<String>,
}

impl InvoiceRequest {
    pub(crate) fn has(&self, flag: u64) -> bool {
        self.flags & flag != 0
    }
}

fn read_transfer_destination(reader: &mut Reader<'_>) -> Result<InvoiceDestination, WalletError> {
    let kind = reader.u8()?;
    let bytes = reader.var_slice()?;
    let mut supported = true;
    if kind & FLAG_DEST_GATEWAY != 0 {
        reader.hash160()?;
        reader.hash160()?;
        reader.i64_le()?;
        supported = false;
    }
    if kind & FLAG_DEST_AUX != 0 {
        let count = reader.count()?;
        for _ in 0..count {
            reader.var_slice()?;
        }
        supported = false;
    }
    let hash: Option<[u8; 20]> = bytes.try_into().ok();
    Ok(match (supported, kind, hash) {
        (true, DEST_PKH, Some(hash)) => {
            InvoiceDestination::Transparent(base58_check(R_ADDRESS_VERSION, &hash))
        }
        (true, DEST_ID, Some(hash)) => InvoiceDestination::Transparent(i_address(&hash)),
        _ => InvoiceDestination::Unsupported,
    })
}

/// `VerusPayInvoiceDetails` at VerusPay version 4, which generic requests use:
/// numbers are CompactSize values.
pub(crate) fn parse_invoice(data: &[u8]) -> Result<InvoiceRequest, WalletError> {
    let mut reader = Reader::new(data);
    let flags = reader.compact_size()?;
    let amount = if flags & INVOICE_ACCEPTS_ANY_AMOUNT == 0 {
        Some(reader.compact_size()?)
    } else {
        None
    };
    let destination = if flags & INVOICE_ACCEPTS_ANY_DESTINATION == 0 {
        Some(if flags & INVOICE_DESTINATION_IS_SAPLING != 0 {
            let raw: [u8; 43] = reader.take(43)?.try_into().map_err(|_| invalid())?;
            InvoiceDestination::Private(sapling_address(&raw).ok_or_else(invalid)?)
        } else {
            read_transfer_destination(&mut reader)?
        })
    } else {
        None
    };
    let requested_currency_id = i_address(&reader.hash160()?);
    let expiry_height = if flags & INVOICE_EXPIRES != 0 {
        Some(reader.compact_size()?)
    } else {
        None
    };
    if flags & INVOICE_ACCEPTS_CONVERSION != 0 {
        let _max_estimated_slippage = reader.compact_size()?;
    }
    let mut accepted_systems = Vec::new();
    if flags & INVOICE_ACCEPTS_NON_VERUS_SYSTEMS != 0 {
        let count = reader.count()?;
        for _ in 0..count {
            accepted_systems.push(i_address(&reader.hash160()?));
        }
    }
    if flags & INVOICE_IS_TAGGED != 0 {
        CompactAddress::read(&mut reader)?;
    }
    reader.finish()?;
    Ok(InvoiceRequest {
        flags,
        amount,
        destination,
        requested_currency_id,
        expiry_height,
        accepted_systems,
    })
}

// --- Sign data ---------------------------------------------------------------

const DATA_PACKET_FLAG_HAS_REQUEST_ID: u64 = 1;
const DATA_PACKET_FLAG_HAS_STATEMENTS: u64 = 2;
const DATA_PACKET_FLAG_HAS_SIGNATURE: u64 = 4;
pub(crate) const DATA_PACKET_FLAG_FOR_USERS_SIGNATURE: u64 = 8;
pub(crate) const DATA_PACKET_FLAG_FOR_TRANSMITTAL_TO_USER: u64 = 16;
const SIGNABLE_OBJECT_TYPE_DATA_DESCRIPTOR: u64 = 0;
const SIGNABLE_OBJECT_TYPE_STRING: u64 = 1;

const DESCRIPTOR_FLAG_SALT: u64 = 2;
const DESCRIPTOR_FLAG_EPK: u64 = 4;
const DESCRIPTOR_FLAG_IVK: u64 = 8;
const DESCRIPTOR_FLAG_SSK: u64 = 0x10;
const DESCRIPTOR_FLAG_LABEL: u64 = 0x20;
const DESCRIPTOR_FLAG_MIME: u64 = 0x40;
const DESCRIPTOR_FLAG_VDXF_KEY: u64 = 0x80;
/// `readLimitedString` limits in `DataDescriptor.fromBuffer`.
const DESCRIPTOR_LABEL_MAX: usize = 64;
const DESCRIPTOR_MIME_MAX: usize = 128;

/// The fields of a `DataDescriptor` that the wallet reads.
#[derive(Debug, Clone, Copy)]
pub(crate) struct DataDescriptor<'a> {
    pub objectdata: &'a [u8],
    pub epk: Option<&'a [u8]>,
    pub ivk: Option<&'a [u8]>,
    pub ssk: Option<&'a [u8]>,
    /// The descriptor's exact bytes.
    pub bytes: &'a [u8],
}

/// Reads a `DataDescriptor` (`DataDescriptor.fromBuffer`).
pub(crate) fn parse_data_descriptor<'a>(
    reader: &mut Reader<'a>,
) -> Result<DataDescriptor<'a>, WalletError> {
    let start = reader.position();
    let _version = reader.varint()?;
    let flags = reader.varint()?;
    if flags & DESCRIPTOR_FLAG_VDXF_KEY != 0 {
        reader.hash160()?;
    }
    let objectdata = reader.var_slice()?;
    if flags & DESCRIPTOR_FLAG_LABEL != 0 && reader.var_slice()?.len() > DESCRIPTOR_LABEL_MAX {
        return Err(invalid());
    }
    if flags & DESCRIPTOR_FLAG_MIME != 0 && reader.var_slice()?.len() > DESCRIPTOR_MIME_MAX {
        return Err(invalid());
    }
    let mut optional = |flag: u64| -> Result<Option<&'a [u8]>, WalletError> {
        if flags & flag != 0 {
            reader.var_slice().map(Some)
        } else {
            Ok(None)
        }
    };
    let _salt = optional(DESCRIPTOR_FLAG_SALT)?;
    let epk = optional(DESCRIPTOR_FLAG_EPK)?;
    let ivk = optional(DESCRIPTOR_FLAG_IVK)?;
    let ssk = optional(DESCRIPTOR_FLAG_SSK)?;
    Ok(DataDescriptor {
        objectdata,
        epk,
        ivk,
        ssk,
        bytes: reader.since(start),
    })
}

/// Reads a `DataDescriptor` and returns its exact bytes.
pub(crate) fn read_data_descriptor<'a>(reader: &mut Reader<'a>) -> Result<&'a [u8], WalletError> {
    parse_data_descriptor(reader).map(|descriptor| descriptor.bytes)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DataPacketRequest {
    pub flags: u64,
    /// The bytes signed for each signable object, then for each statement:
    /// a text as UTF-8, a data descriptor as serialized (`signDataPacket.js`).
    pub signable: Vec<Vec<u8>>,
    pub object_count: usize,
    pub request_id: Option<String>,
}

pub(crate) fn parse_data_packet(
    data: &[u8],
    is_testnet: bool,
) -> Result<DataPacketRequest, WalletError> {
    let mut reader = Reader::new(data);
    let flags = reader.compact_size()?;
    let object_count = reader.count()?;
    let mut signable = Vec::with_capacity(object_count);
    for _ in 0..object_count {
        match reader.compact_size()? {
            SIGNABLE_OBJECT_TYPE_STRING => {
                let text = reader.var_slice()?;
                std::str::from_utf8(text).map_err(|_| invalid())?;
                signable.push(text.to_vec());
            }
            SIGNABLE_OBJECT_TYPE_DATA_DESCRIPTOR => {
                signable.push(read_data_descriptor(&mut reader)?.to_vec());
            }
            _ => return Err(invalid()),
        }
    }
    if flags & DATA_PACKET_FLAG_HAS_STATEMENTS != 0 {
        let count = reader.count()?;
        for _ in 0..count {
            let statement = reader.var_slice()?;
            std::str::from_utf8(statement).map_err(|_| invalid())?;
            signable.push(statement.to_vec());
        }
    }
    if flags & DATA_PACKET_FLAG_HAS_SIGNATURE != 0 {
        let start = reader.position();
        let (_, end) = crate::core::crypto::verus_id_signature::parse_verifiable_signature_data(
            data, start, is_testnet,
        )?;
        reader.take(end - start)?;
    }
    let request_id = if flags & DATA_PACKET_FLAG_HAS_REQUEST_ID != 0 {
        Some(CompactAddress::read(&mut reader)?.identity_address(is_testnet)?)
    } else {
        None
    };
    reader.finish()?;
    Ok(DataPacketRequest {
        flags,
        signable,
        object_count,
        request_id,
    })
}

// --- Share data --------------------------------------------------------------

const USER_DATA_FLAG_HAS_REQUEST_ID: u64 = 1;
const USER_DATA_FLAG_HAS_SIGNER: u64 = 2;
const USER_DATA_FLAG_HAS_REQUESTED_KEYS: u64 = 4;
pub(crate) const USER_DATA_FULL_DATA: u64 = 1;
const USER_DATA_COLLECTION: u64 = 3;
pub(crate) const USER_DATA_CREDENTIAL: u64 = 3;
const USER_DATA_ATTESTATION: u64 = 1;
/// `HASH_TYPE_BLAKE2B` (1) to `HASH_TYPE_SHA256` (5).
const HASH_TYPES: std::ops::RangeInclusive<u64> = 1..=5;

/// `UserDataRequestDetails`: what data the app asks for, and from whom.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UserDataRequest {
    pub data_type: u64,
    pub request_type: u64,
    pub hash_type: u64,
    /// The data keys the app asks for, each with its search value.
    pub search_keys: Vec<(String, Vec<u8>)>,
    /// The identity that must answer, when the app names one.
    pub signer: Option<String>,
    pub requested_keys: Vec<String>,
    pub request_id: Option<String>,
}

impl UserDataRequest {
    /// `UserDataRequestDetails.isValid()`.
    pub(crate) fn is_valid(&self) -> bool {
        (USER_DATA_FULL_DATA..=USER_DATA_COLLECTION).contains(&self.data_type)
            && (USER_DATA_ATTESTATION..=USER_DATA_CREDENTIAL).contains(&self.request_type)
            && HASH_TYPES.contains(&self.hash_type)
            && !self.search_keys.is_empty()
    }
}

pub(crate) fn parse_user_data(
    data: &[u8],
    is_testnet: bool,
) -> Result<UserDataRequest, WalletError> {
    let mut reader = Reader::new(data);
    let flags = reader.compact_size()?;
    let data_type = reader.compact_size()?;
    let request_type = reader.compact_size()?;
    let hash_type = reader.compact_size()?;
    let count = reader.count()?;
    let mut search_keys = Vec::with_capacity(count);
    for _ in 0..count {
        let key = i_address(&reader.hash160()?);
        search_keys.push((key, reader.var_slice()?.to_vec()));
    }
    let signer = if flags & USER_DATA_FLAG_HAS_SIGNER != 0 {
        Some(CompactAddress::read(&mut reader)?.identity_address(is_testnet)?)
    } else {
        None
    };
    let mut requested_keys = Vec::new();
    if flags & USER_DATA_FLAG_HAS_REQUESTED_KEYS != 0 {
        let count = reader.count()?;
        for _ in 0..count {
            requested_keys.push(i_address(&reader.hash160()?));
        }
    }
    let request_id = if flags & USER_DATA_FLAG_HAS_REQUEST_ID != 0 {
        Some(CompactAddress::read(&mut reader)?.identity_address(is_testnet)?)
    } else {
        None
    };
    reader.finish()?;
    Ok(UserDataRequest {
        data_type,
        request_type,
        hash_type,
        search_keys,
        signer,
        requested_keys,
        request_id,
    })
}

// --- Encryption key ----------------------------------------------------------

const APP_ENCRYPTION_FLAG_HAS_REQUEST_ID: u64 = 1;
const APP_ENCRYPTION_FLAG_HAS_ENCRYPT_TO: u64 = 2;
const APP_ENCRYPTION_FLAG_HAS_DERIVATION_ID: u64 = 4;
const APP_ENCRYPTION_FLAG_RETURN_SPENDING_KEY: u64 = 8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AppEncryptionRequest {
    /// The address to encrypt this detail's response to.
    pub encrypt_to: Option<[u8; 43]>,
    pub derivation_number: u64,
    pub derivation_id: Option<String>,
    pub request_id: Option<String>,
    /// The app asks for the channel's extended spending key.
    pub return_spending_key: bool,
}

pub(crate) fn parse_app_encryption(
    data: &[u8],
    is_testnet: bool,
) -> Result<AppEncryptionRequest, WalletError> {
    let mut reader = Reader::new(data);
    let flags = reader.compact_size()?;
    let encrypt_to = if flags & APP_ENCRYPTION_FLAG_HAS_ENCRYPT_TO != 0 {
        Some(reader.take(43)?.try_into().map_err(|_| invalid())?)
    } else {
        None
    };
    let derivation_number = reader.varint()?;
    let derivation_id = if flags & APP_ENCRYPTION_FLAG_HAS_DERIVATION_ID != 0 {
        Some(CompactAddress::read(&mut reader)?.identity_address(is_testnet)?)
    } else {
        None
    };
    let request_id = if flags & APP_ENCRYPTION_FLAG_HAS_REQUEST_ID != 0 {
        Some(CompactAddress::read(&mut reader)?.identity_address(is_testnet)?)
    } else {
        None
    };
    reader.finish()?;
    Ok(AppEncryptionRequest {
        encrypt_to,
        derivation_number,
        derivation_id,
        request_id,
        return_spending_key: flags & APP_ENCRYPTION_FLAG_RETURN_SPENDING_KEY != 0,
    })
}
