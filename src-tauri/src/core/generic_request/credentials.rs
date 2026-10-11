//! Credentials that an identity keeps encrypted in its content, as Verus
//! Mobile writes and reads them at `dev-asf-parent` (`encryptCredentials.js`,
//! `scopedCredentials.js`, `buildUserDataResponse.js`).
//!
//! An identity stores its credentials under a key derived from its own
//! encryption channel (from the identity to itself, index 0), encrypted to
//! that channel's address. Only a wallet with the identity's private-address
//! seed can find and read them.

use serde_json::{json, Map, Value};
use zeroize::Zeroizing;

use crate::core::crypto::verus_id_signature::{write_compact_size, write_varint};
use crate::types::WalletError;

use super::details::parse_data_descriptor;
use super::names::{decode_i_address, hash160, i_address, sha256d};
use super::private_crypto::{decrypt_with_symmetric_key, decrypt_with_viewing_key};
use super::request::CompactAddress;
use super::response::{
    encrypt_wrapped_data, DATA_DESCRIPTOR_KEY, DATA_DESCRIPTOR_VERSION,
    DESCRIPTOR_FLAG_ENCRYPTED_DATA, DESCRIPTOR_FLAG_EPK_PRESENT,
};
use super::wire::{invalid, Reader};

/// `vrsc::identity.credential`: the content key a request writes credentials
/// under. The wallet stores them under a key hashed from it instead.
pub(crate) const IDENTITY_CREDENTIAL_KEY: &str = "iMRGoNZvSuayeu17PmbXv2CA6eRL4xfmhm";
/// `vrsc::data.type.object.credential` (`CredentialKey`).
pub(crate) const CREDENTIAL_KEY: &str = "iDTG49YLqmkHMYRyuQBYgEyTByQwAzqGd6";
const CREDENTIAL_KEY_NAME: &str = "vrsc::data.type.object.credential";
const DATA_DESCRIPTOR_KEY_NAME: &str = "vrsc::data.type.object.datadescriptor";
/// `NULL_ADDRESS`: a credential under this key is not valid.
const NULL_KEY: &str = "i3UXS5QPRQGNRDDqVnyWTnmFCTHDbzmsYk";
const CREDENTIAL_VERSION: u64 = 1;
const CREDENTIAL_FLAG_LABEL: u64 = 1;
/// `Credential.MAX_JSON_STRING_LENGTH`.
const CREDENTIAL_TEXT_MAX: usize = 512;

/// One credential, as `Credential.fromBuffer` reads it.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Credential {
    pub version: u64,
    pub key: String,
    pub credential: Value,
    pub scopes: Value,
    pub label: Option<String>,
    /// The serialized credential, which the response carries unchanged.
    pub bytes: Vec<u8>,
}

impl Credential {
    /// `Credential.isValid()`.
    fn is_valid(&self) -> bool {
        let text_fits = |value: &Value| {
            serde_json::to_string(value).is_ok_and(|text| text.len() <= CREDENTIAL_TEXT_MAX)
        };
        self.version == CREDENTIAL_VERSION
            && self.key != NULL_KEY
            && text_fits(&self.credential)
            && text_fits(&self.scopes)
            && self
                .label
                .as_ref()
                .is_none_or(|label| label.len() <= CREDENTIAL_TEXT_MAX)
    }

    /// Whether the credential may go to `scope` (`scopedCredentials.js`): the
    /// scope is one of its scopes, a key or value of a scope object, or the
    /// scope itself.
    fn allows(&self, scope: &str) -> bool {
        let is_scope = |value: &Value| value.as_str() == Some(scope);
        match &self.scopes {
            Value::Array(scopes) => scopes.iter().any(is_scope),
            Value::Object(scopes) => scopes.contains_key(scope) || scopes.values().any(is_scope),
            value => is_scope(value),
        }
    }
}

fn limited_text<'a>(reader: &mut Reader<'a>) -> Result<&'a [u8], WalletError> {
    let text = reader.var_slice()?;
    if text.len() > CREDENTIAL_TEXT_MAX {
        return Err(invalid());
    }
    Ok(text)
}

pub(crate) fn parse_credential(bytes: &[u8]) -> Result<Credential, WalletError> {
    let mut reader = Reader::new(bytes);
    let version = reader.varint()?;
    let flags = reader.varint()?;
    let key = i_address(&reader.hash160()?);
    let credential = serde_json::from_slice(limited_text(&mut reader)?).map_err(|_| invalid())?;
    let scopes = serde_json::from_slice(limited_text(&mut reader)?).map_err(|_| invalid())?;
    let label = if flags & CREDENTIAL_FLAG_LABEL != 0 {
        Some(String::from_utf8(limited_text(&mut reader)?.to_vec()).map_err(|_| invalid())?)
    } else {
        None
    };
    reader.finish()?;
    Ok(Credential {
        version,
        key,
        credential,
        scopes,
        label,
        bytes: bytes.to_vec(),
    })
}

/// The key the credentials of an identity are stored under
/// (`getConditionID`): HASH160 of SHA-256d over the credential key and the
/// channel's incoming viewing key. Verus Mobile reverses the key bytes it
/// gets from its native module, which are the scalar's own byte order.
pub(crate) fn storage_key(incoming_viewing_key: &[u8; 32]) -> Result<String, WalletError> {
    let key = decode_i_address(IDENTITY_CREDENTIAL_KEY).ok_or_else(invalid)?;
    let mut reversed = *incoming_viewing_key;
    reversed.reverse();
    Ok(i_address(&hash160(&sha256d(&[&key, &reversed]))))
}

/// One `VdxfUniValue` entry of an object type: its key, version, and payload.
struct UniEntry<'a> {
    key: CompactAddress,
    version: u64,
    payload: &'a [u8],
}

impl UniEntry<'_> {
    fn is(&self, key: &str, name: &str) -> bool {
        match &self.key {
            CompactAddress::IAddress(hash) => i_address(hash) == key,
            CompactAddress::Name(text) => text.eq_ignore_ascii_case(name),
            CompactAddress::XAddress(_) => false,
        }
    }
}

/// The object entries of a `VdxfUniValue`. In the `FqnVdxfUniValue` form
/// that identity requests carry, each key is a compact address; else it is
/// a 20-byte key.
fn uni_entries(bytes: &[u8], compact_keys: bool) -> Result<Vec<UniEntry<'_>>, WalletError> {
    let mut reader = Reader::new(bytes);
    let mut entries = Vec::new();
    while !reader.is_done() {
        let key = if compact_keys {
            CompactAddress::read(&mut reader)?
        } else {
            CompactAddress::IAddress(reader.hash160()?)
        };
        let version = reader.varint()?;
        let length = reader.count()?;
        entries.push(UniEntry {
            key,
            version,
            payload: reader.take(length)?,
        });
    }
    Ok(entries)
}

fn credential_entry(version: u64, credential: &[u8]) -> Result<Vec<u8>, WalletError> {
    let mut out = decode_i_address(CREDENTIAL_KEY)
        .ok_or_else(invalid)?
        .to_vec();
    write_varint(&mut out, version);
    write_compact_size(&mut out, credential.len());
    out.extend_from_slice(credential);
    Ok(out)
}

/// The credential response's `VdxfUniValue`: each credential under the
/// credential key (`buildUserDataResponse.js`).
pub(crate) fn credential_values(credentials: &[Credential]) -> Result<Vec<u8>, WalletError> {
    let mut out = Vec::new();
    for credential in credentials {
        out.extend_from_slice(&credential_entry(credential.version, &credential.bytes)?);
    }
    Ok(out)
}

/// `processEncryptedKeys`: each credential a request writes under
/// `vrsc::identity.credential` becomes its own value, encrypted to the
/// identity's credential address, in the daemon's data descriptor JSON.
pub(crate) fn encrypt_for_storage(
    values: &[Vec<u8>],
    address: &[u8; 43],
) -> Result<Vec<Value>, WalletError> {
    let mut out = Vec::new();
    for value in values {
        for entry in uni_entries(value, true)? {
            if !entry.is(CREDENTIAL_KEY, CREDENTIAL_KEY_NAME) {
                return Err(WalletError::IdentityBuildFailed);
            }
            parse_credential(entry.payload)?;
            let plain = Zeroizing::new(credential_entry(entry.version, entry.payload)?);
            let payload = encrypt_wrapped_data(address, &plain)?;
            out.push(json!({
                DATA_DESCRIPTOR_KEY: {
                    "version": DATA_DESCRIPTOR_VERSION,
                    "flags": DESCRIPTOR_FLAG_ENCRYPTED_DATA | DESCRIPTOR_FLAG_EPK_PRESENT,
                    "objectdata": hex::encode(&payload.ciphertext),
                    "epk": hex::encode(payload.ephemeral_public_key),
                }
            }));
        }
    }
    Ok(out)
}

/// An encrypted data descriptor as stored in identity content.
struct StoredDescriptor {
    objectdata: Vec<u8>,
    epk: Option<Vec<u8>>,
    ivk: Option<Vec<u8>>,
    ssk: Option<Vec<u8>>,
}

fn hex_field(object: &Map<String, Value>, field: &str) -> Option<Vec<u8>> {
    object
        .get(field)
        .and_then(Value::as_str)
        .and_then(|value| hex::decode(value).ok())
}

/// `getDescriptorFromStoredValue`: the daemon returns a stored value as JSON
/// when it can read it, else as hex.
fn stored_descriptor(value: &Value) -> Option<StoredDescriptor> {
    match value {
        Value::String(raw) => {
            let bytes = hex::decode(raw).ok()?;
            let entries = uni_entries(&bytes, true)
                .ok()
                .filter(|entries| !entries.is_empty())
                .or_else(|| uni_entries(&bytes, false).ok())?;
            let entry = entries
                .iter()
                .find(|entry| entry.is(DATA_DESCRIPTOR_KEY, DATA_DESCRIPTOR_KEY_NAME))?;
            let mut reader = Reader::new(entry.payload);
            let descriptor = parse_data_descriptor(&mut reader).ok()?;
            Some(StoredDescriptor {
                objectdata: descriptor.objectdata.to_vec(),
                epk: descriptor.epk.map(<[u8]>::to_vec),
                ivk: descriptor.ivk.map(<[u8]>::to_vec),
                ssk: descriptor.ssk.map(<[u8]>::to_vec),
            })
        }
        Value::Object(object) => {
            let descriptor = object
                .get(DATA_DESCRIPTOR_KEY)
                .or_else(|| object.get(DATA_DESCRIPTOR_KEY_NAME))
                .and_then(Value::as_object)
                .or_else(|| object.contains_key("objectdata").then_some(object))?;
            let objectdata = match descriptor.get("objectdata")? {
                Value::String(raw) => hex::decode(raw).ok()?,
                Value::Object(inner) => hex_field(inner, "serializedhex")?,
                _ => return None,
            };
            Some(StoredDescriptor {
                objectdata,
                epk: hex_field(descriptor, "epk"),
                ivk: hex_field(descriptor, "ivk"),
                ssk: hex_field(descriptor, "ssk"),
            })
        }
        _ => None,
    }
}

/// `extractCredentialsFromPlaintext`: credentials directly under the
/// credential key, or inside a data descriptor.
fn credentials_in(plaintext: &[u8]) -> Result<Vec<Credential>, WalletError> {
    let mut credentials = Vec::new();
    for entry in uni_entries(plaintext, false)? {
        if entry.is(CREDENTIAL_KEY, CREDENTIAL_KEY_NAME) {
            credentials.push(parse_credential(entry.payload)?);
        } else if entry.is(DATA_DESCRIPTOR_KEY, DATA_DESCRIPTOR_KEY_NAME) {
            let mut reader = Reader::new(entry.payload);
            let descriptor = parse_data_descriptor(&mut reader)?;
            for inner in uni_entries(descriptor.objectdata, false)? {
                if inner.is(CREDENTIAL_KEY, CREDENTIAL_KEY_NAME) {
                    credentials.push(parse_credential(inner.payload)?);
                }
            }
        }
    }
    Ok(credentials)
}

fn decrypt_stored(
    descriptor: &StoredDescriptor,
    incoming_viewing_key: &[u8; 32],
) -> Result<Zeroizing<Vec<u8>>, WalletError> {
    if let Some(symmetric_key) = &descriptor.ssk {
        return decrypt_with_symmetric_key(symmetric_key, &descriptor.objectdata);
    }
    let ivk: [u8; 32] = match &descriptor.ivk {
        Some(ivk) => ivk.as_slice().try_into().map_err(|_| invalid())?,
        None => *incoming_viewing_key,
    };
    let epk: [u8; 32] = descriptor
        .epk
        .as_deref()
        .ok_or_else(invalid)?
        .try_into()
        .map_err(|_| invalid())?;
    decrypt_with_viewing_key(&ivk, &epk, &descriptor.objectdata)
}

/// `getScopedCredentials`: the valid credentials in `stored` whose key was
/// requested and whose scopes include `scope`. As on Verus Mobile, a value
/// that does not decrypt or parse is skipped: it cannot hold a credential
/// this wallet wrote.
pub(crate) fn read_credentials(
    stored: &[Value],
    incoming_viewing_key: &[u8; 32],
    requested_keys: &[String],
    scope: &str,
) -> Vec<Credential> {
    let mut found = Vec::new();
    for value in stored {
        let Some(descriptor) = stored_descriptor(value) else {
            continue;
        };
        let Ok(plaintext) = decrypt_stored(&descriptor, incoming_viewing_key) else {
            continue;
        };
        let Ok(credentials) = credentials_in(&plaintext) else {
            continue;
        };
        found.extend(credentials.into_iter().filter(|credential| {
            requested_keys.contains(&credential.key)
                && credential.allows(scope)
                && credential.is_valid()
        }));
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::generic_request::private_crypto::channel_keys;

    fn vectors() -> Value {
        let raw = include_str!("../../../../src/lib/genericRequest/fixtures/request-vectors.json");
        serde_json::from_str(raw).expect("vectors parse")
    }

    fn credentials_case() -> Value {
        vectors()["credentials"].clone()
    }

    fn account() -> sapling::zip32::ExtendedSpendingKey {
        let zfunc: Value =
            serde_json::from_str(include_str!("fixtures/verus-zfunc-vectors.json")).unwrap();
        let bytes = hex::decode(zfunc["accountExtsk"].as_str().unwrap()).unwrap();
        sapling::zip32::ExtendedSpendingKey::from_bytes(&bytes)
            .ok()
            .expect("account key parses")
    }

    #[test]
    fn the_storage_key_matches_verus_mobile() {
        let case = credentials_case();
        let ivk: [u8; 32] = hex::decode(case["ivkRaw"].as_str().unwrap())
            .unwrap()
            .try_into()
            .unwrap();
        assert_eq!(
            storage_key(&ivk).unwrap(),
            case["storageKey"].as_str().unwrap()
        );
    }

    #[test]
    fn credentials_written_for_an_identity_read_back_by_key_and_scope() {
        let case = credentials_case();
        let identity = decode_i_address(case["identity"].as_str().unwrap()).unwrap();
        let keys = channel_keys(&account(), Some(&identity), Some(&identity), 0, false).unwrap();
        let request_values = case["requestValues"]
            .as_array()
            .unwrap()
            .iter()
            .map(|value| hex::decode(value.as_str().unwrap()).unwrap())
            .collect::<Vec<_>>();

        let stored = encrypt_for_storage(&request_values, &keys.address.to_bytes()).unwrap();
        assert_eq!(stored.len(), 2);
        assert!(!serde_json::to_string(&stored)
            .unwrap()
            .contains("Deep Link Sim test profile"));

        let scope = case["scope"].as_str().unwrap();
        let profile = case["profileKey"].as_str().unwrap().to_string();
        let membership = case["membershipKey"].as_str().unwrap().to_string();
        let found = read_credentials(
            &stored,
            &keys.incoming_viewing_key,
            &[profile.clone(), membership.clone()],
            scope,
        );
        assert_eq!(
            found
                .iter()
                .map(|credential| &credential.key)
                .collect::<Vec<_>>(),
            vec![&profile, &membership]
        );
        assert_eq!(found[0].label.as_deref(), Some("Test profile"));

        // Another key, another scope, or another identity's key finds nothing.
        assert_eq!(
            read_credentials(
                &stored,
                &keys.incoming_viewing_key,
                &[profile.clone()],
                scope
            )
            .len(),
            1
        );
        assert!(read_credentials(
            &stored,
            &keys.incoming_viewing_key,
            &[profile.clone()],
            "iJhCezBExJHvtyH3fGhNnt2NhU4Ztkf2yq"
        )
        .is_empty());
        let other = channel_keys(&account(), Some(&[7; 20]), Some(&[7; 20]), 0, false).unwrap();
        assert!(
            read_credentials(&stored, &other.incoming_viewing_key, &[profile], scope).is_empty()
        );
    }

    #[test]
    fn a_stored_value_in_hex_form_reads_like_its_json_form() {
        let case = credentials_case();
        let identity = decode_i_address(case["identity"].as_str().unwrap()).unwrap();
        let keys = channel_keys(&account(), Some(&identity), Some(&identity), 0, false).unwrap();
        let request_value = hex::decode(case["requestValues"][0].as_str().unwrap()).unwrap();
        let stored = encrypt_for_storage(&[request_value], &keys.address.to_bytes()).unwrap();
        let descriptor = &stored[0][DATA_DESCRIPTOR_KEY];

        // The daemon's bytes for the value: the data descriptor key, its
        // version, and the serialized descriptor.
        let mut inner = Vec::new();
        write_varint(&mut inner, 1);
        write_varint(&mut inner, descriptor["flags"].as_u64().unwrap());
        crate::core::crypto::verus_id_signature::write_var_slice(
            &mut inner,
            &hex::decode(descriptor["objectdata"].as_str().unwrap()).unwrap(),
        );
        crate::core::crypto::verus_id_signature::write_var_slice(
            &mut inner,
            &hex::decode(descriptor["epk"].as_str().unwrap()).unwrap(),
        );
        let mut value = decode_i_address(DATA_DESCRIPTOR_KEY).unwrap().to_vec();
        write_varint(&mut value, 1);
        write_compact_size(&mut value, inner.len());
        value.extend_from_slice(&inner);

        let found = read_credentials(
            &[Value::String(hex::encode(value))],
            &keys.incoming_viewing_key,
            &[case["profileKey"].as_str().unwrap().to_string()],
            case["scope"].as_str().unwrap(),
        );
        assert_eq!(found.len(), 1);
    }

    #[test]
    fn a_value_with_another_object_than_a_credential_is_refused() {
        let case = credentials_case();
        let identity = decode_i_address(case["identity"].as_str().unwrap()).unwrap();
        let keys = channel_keys(&account(), Some(&identity), Some(&identity), 0, false).unwrap();
        let mut value = hex::decode(case["requestValues"][0].as_str().unwrap()).unwrap();
        // The compact key's last hash byte changes, so it names another key.
        let key_end = 2 + 20;
        value[key_end - 1] ^= 1;
        assert!(encrypt_for_storage(&[value], &keys.address.to_bytes()).is_err());
    }
}
