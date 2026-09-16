use serde_json::Value;
use unicode_segmentation::UnicodeSegmentation;

use crate::core::channels::vrpc::identity::profile::codec::{
    avatar_base64, descriptor_from_json, resolve_profile_payload, validate_avatar_bytes,
    DataDescriptor, AVATAR_MIME, AVATAR_VDXF_KEY, DESCRIPTION_MIME, DESCRIPTION_VDXF_KEY,
    MAX_DESCRIPTION_BYTES,
};
use crate::core::channels::vrpc::provider::VrpcProvider;
use crate::core::crypto::wif_encoding::Network;
use crate::types::{
    IdentityProfileAvatar, IdentityProfileField, IdentityProfileIssue, IdentityProfileLoadResult,
    IdentityProfileSource, IdentityProfileState, WalletError,
};

const MAX_PROFILE_JSON_BYTES: usize = 2 * 1024 * 1024;
const MAX_HISTORY_RECORDS: usize = 256;
const MAX_DESCRIPTION_GRAPHEMES: usize = 160;

#[derive(Clone)]
struct Origin {
    txid: String,
    identity_vout: u32,
    height: u32,
    blockhash: String,
    system_id: String,
    identity_id: String,
    primary_addresses: Vec<String>,
}

fn bounded(value: Value) -> Result<Value, WalletError> {
    if serde_json::to_vec(&value)
        .map_err(|_| WalletError::IdentityProfileUnavailable)?
        .len()
        > MAX_PROFILE_JSON_BYTES
    {
        return Err(WalletError::IdentityProfileUnavailable);
    }
    Ok(value)
}

fn as_u32(value: Option<&Value>) -> Option<u32> {
    value?
        .as_u64()
        .and_then(|number| u32::try_from(number).ok())
}

fn non_empty(value: Option<&Value>) -> Option<String> {
    let value = value?.as_str()?.trim();
    (!value.is_empty()).then(|| value.to_string())
}

fn valid_txid(value: Option<&Value>) -> Option<String> {
    let value = value?.as_str()?.trim();
    (value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .then(|| value.to_ascii_lowercase())
}

fn matching_revision_txid(avatar_content: &Value, description_content: &Value) -> Option<String> {
    // Both independently filtered reads must identify the same canonical
    // identity revision. One missing or conflicting claim cannot confirm a
    // pending profile update.
    let avatar_txid = valid_txid(avatar_content.get("txid"))?;
    let description_txid = valid_txid(description_content.get("txid"))?;
    (avatar_txid == description_txid).then_some(avatar_txid)
}

fn content_values<'a>(payload: &'a Value, key: &str) -> Vec<&'a Value> {
    payload
        .pointer("/identity/contentmultimap")
        .or_else(|| payload.pointer("/identity/contentMultiMap"))
        .and_then(Value::as_object)
        .and_then(|map| map.get(key))
        .and_then(Value::as_array)
        .map(|values| values.iter().collect())
        .unwrap_or_default()
}

pub(crate) async fn active_values(
    provider: &VrpcProvider,
    identity_address: &str,
    key: &str,
) -> Result<Vec<Value>, WalletError> {
    let content = bounded(
        provider
            .getidentitycontent_for_key(identity_address, 0, 0, false, key)
            .await?,
    )?;
    Ok(content_values(&content, key).into_iter().cloned().collect())
}

fn descriptor_group(value: &Value) -> Result<Vec<DataDescriptor>, WalletError> {
    let entries = value
        .as_array()
        .filter(|entries| entries.len() == 2)
        .ok_or(WalletError::IdentityProfileUnavailable)?;
    entries
        .iter()
        .map(|entry| {
            let descriptor = entry
                .get("i4GC1YGEVD21afWudGoFJVdnfjJ5XWnCQv")
                .ok_or(WalletError::IdentityProfileUnavailable)?;
            descriptor_from_json(descriptor)
        })
        .collect()
}

fn origin_for_value(history: &Value, key: &str, selected: &Value) -> Option<Origin> {
    let entries = history.get("history")?.as_array()?;
    for entry in entries.iter().rev() {
        let Some(values) = entry
            .pointer("/identity/contentmultimap")
            .or_else(|| entry.pointer("/identity/contentMultiMap"))
            .and_then(Value::as_object)
            .and_then(|map| map.get(key))
            .and_then(Value::as_array)
        else {
            continue;
        };
        if !values.iter().any(|value| value == selected) {
            continue;
        }
        let identity = entry.get("identity")?;
        return Some(Origin {
            txid: non_empty(entry.pointer("/output/txid"))?,
            identity_vout: as_u32(entry.pointer("/output/voutnum"))?,
            height: as_u32(entry.get("height"))?,
            blockhash: non_empty(entry.get("blockhash"))?,
            system_id: non_empty(
                identity
                    .get("systemid")
                    .or_else(|| identity.get("systemId")),
            )?,
            identity_id: non_empty(
                identity
                    .get("identityaddress")
                    .or_else(|| identity.get("identityAddress")),
            )?,
            primary_addresses: identity
                .get("primaryaddresses")
                .or_else(|| identity.get("primaryAddresses"))
                .and_then(Value::as_array)?
                .iter()
                .filter_map(|entry| non_empty(Some(entry)))
                .collect(),
        });
    }
    None
}

async fn confirm_origin(provider: &VrpcProvider, origin: &Origin) -> Result<(), WalletError> {
    let header = bounded(
        provider
            .getblockheader_for_profile(&origin.blockhash)
            .await?,
    )?;
    if as_u32(header.get("height")) != Some(origin.height)
        || header
            .get("confirmations")
            .and_then(Value::as_i64)
            .unwrap_or_default()
            <= 0
    {
        return Err(WalletError::IdentityProfileUnavailable);
    }
    Ok(())
}

fn source(origin: &Origin, digest: [u8; 32]) -> IdentityProfileSource {
    IdentityProfileSource {
        system_id: origin.system_id.clone(),
        txid: origin.txid.clone(),
        vout: origin.identity_vout,
        height: origin.height,
        blockhash: origin.blockhash.clone(),
        digest: hex::encode(digest),
    }
}

async fn resolve_avatar(
    provider: &VrpcProvider,
    history: &Value,
    selected: &Value,
    network: Network,
) -> Result<IdentityProfileField<IdentityProfileAvatar>, WalletError> {
    let origin = origin_for_value(history, AVATAR_VDXF_KEY, selected)
        .ok_or(WalletError::IdentityProfileUnavailable)?;
    confirm_origin(provider, &origin).await?;
    let tx = bounded(
        provider
            .getrawtransaction_for_profile(&origin.txid, 1)
            .await?,
    )?;
    let payload = resolve_profile_payload(
        &descriptor_group(selected)?,
        &tx,
        AVATAR_MIME,
        &origin.system_id,
        &origin.identity_id,
        &origin.primary_addresses,
        network,
    )?;
    validate_avatar_bytes(&payload.bytes)?;
    Ok(IdentityProfileField {
        value: IdentityProfileAvatar {
            base64: avatar_base64(&payload.bytes),
            mime_type: payload.mime_type,
            width: 256,
            height: 256,
            byte_length: payload.bytes.len(),
        },
        source: source(&origin, payload.digest),
    })
}

async fn resolve_description(
    provider: &VrpcProvider,
    history: &Value,
    selected: &Value,
    network: Network,
) -> Result<IdentityProfileField<String>, WalletError> {
    let origin = origin_for_value(history, DESCRIPTION_VDXF_KEY, selected)
        .ok_or(WalletError::IdentityProfileUnavailable)?;
    confirm_origin(provider, &origin).await?;
    let tx = bounded(
        provider
            .getrawtransaction_for_profile(&origin.txid, 1)
            .await?,
    )?;
    let payload = resolve_profile_payload(
        &descriptor_group(selected)?,
        &tx,
        DESCRIPTION_MIME,
        &origin.system_id,
        &origin.identity_id,
        &origin.primary_addresses,
        network,
    )?;
    if payload.bytes.is_empty() || payload.bytes.len() > MAX_DESCRIPTION_BYTES {
        return Err(WalletError::IdentityProfileUnavailable);
    }
    let value =
        String::from_utf8(payload.bytes).map_err(|_| WalletError::IdentityProfileUnavailable)?;
    if value.graphemes(true).count() > MAX_DESCRIPTION_GRAPHEMES {
        return Err(WalletError::IdentityProfileUnavailable);
    }
    Ok(IdentityProfileField {
        value,
        source: source(&origin, payload.digest),
    })
}

pub(crate) async fn load(
    provider: &VrpcProvider,
    identity_address: &str,
    network: Network,
) -> Result<IdentityProfileLoadResult, WalletError> {
    let avatar_content = provider
        .getidentitycontent_for_key(identity_address, 0, 0, false, AVATAR_VDXF_KEY)
        .await?;
    let avatar_content = bounded(avatar_content)?;
    let description_content = provider
        .getidentitycontent_for_key(identity_address, 0, 0, false, DESCRIPTION_VDXF_KEY)
        .await?;
    let description_content = bounded(description_content)?;
    let avatar_values = content_values(&avatar_content, AVATAR_VDXF_KEY);
    let description_values = content_values(&description_content, DESCRIPTION_VDXF_KEY);
    let revision_txid = matching_revision_txid(&avatar_content, &description_content);
    let read_height = as_u32(avatar_content.get("blockheight"))
        .or_else(|| as_u32(description_content.get("blockheight")));

    // An identity with no active profile values is a valid empty profile. Do
    // not make that state depend on history or raw-transaction availability.
    if avatar_values.is_empty() && description_values.is_empty() {
        return Ok(IdentityProfileLoadResult {
            state: IdentityProfileState::Empty,
            avatar: None,
            description: None,
            issues: Vec::new(),
            read_height,
            revision_txid,
        });
    }

    let history = bounded(provider.getidentityhistory(identity_address).await?)?;
    let records = history
        .get("history")
        .and_then(Value::as_array)
        .ok_or(WalletError::IdentityProfileUnavailable)?;
    if records.len() > MAX_HISTORY_RECORDS {
        return Err(WalletError::IdentityProfileUnavailable);
    }

    let mut issues = Vec::new();
    if avatar_values.len() > 1 {
        issues.push(IdentityProfileIssue {
            field: Some("avatar".to_string()),
            code: "multiple_active_values".to_string(),
        });
    }
    if description_values.len() > 1 {
        issues.push(IdentityProfileIssue {
            field: Some("description".to_string()),
            code: "multiple_active_values".to_string(),
        });
    }

    let avatar = match avatar_values.last() {
        Some(value) => match resolve_avatar(provider, &history, value, network).await {
            Ok(value) => Some(value),
            Err(_) => {
                issues.push(IdentityProfileIssue {
                    field: Some("avatar".to_string()),
                    code: "invalid_or_unavailable".to_string(),
                });
                None
            }
        },
        None => None,
    };
    let description = match description_values.last() {
        Some(value) => match resolve_description(provider, &history, value, network).await {
            Ok(value) => Some(value),
            Err(_) => {
                issues.push(IdentityProfileIssue {
                    field: Some("description".to_string()),
                    code: "invalid_or_unavailable".to_string(),
                });
                None
            }
        },
        None => None,
    };

    let state = if avatar.is_some() || description.is_some() {
        IdentityProfileState::Ready
    } else {
        IdentityProfileState::Unavailable
    };
    Ok(IdentityProfileLoadResult {
        state,
        avatar,
        description,
        issues,
        read_height,
        revision_txid,
    })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::matching_revision_txid;

    const TXID: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    #[test]
    fn matching_revision_txid_accepts_the_same_valid_revision_from_both_reads() {
        let avatar = json!({ "txid": TXID.to_ascii_uppercase() });
        let description = json!({ "txid": TXID });

        assert_eq!(
            matching_revision_txid(&avatar, &description),
            Some(TXID.to_string())
        );
    }

    #[test]
    fn matching_revision_txid_rejects_missing_revision_evidence() {
        let avatar = json!({ "txid": TXID });

        assert_eq!(matching_revision_txid(&avatar, &json!({})), None);
    }

    #[test]
    fn matching_revision_txid_rejects_malformed_revision_evidence() {
        let avatar = json!({ "txid": "not-a-transaction-id" });
        let description = json!({ "txid": "not-a-transaction-id" });

        assert_eq!(matching_revision_txid(&avatar, &description), None);
    }

    #[test]
    fn matching_revision_txid_rejects_conflicting_revision_evidence() {
        let avatar = json!({ "txid": TXID });
        let description =
            json!({ "txid": "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff" });

        assert_eq!(matching_revision_txid(&avatar, &description), None);
    }

    #[test]
    fn matching_revision_txid_can_confirm_an_empty_profile_revision() {
        let avatar = json!({ "txid": TXID, "identity": { "contentmultimap": {} } });
        let description = json!({ "txid": TXID, "identity": { "contentmultimap": {} } });

        assert_eq!(
            matching_revision_txid(&avatar, &description),
            Some(TXID.to_string())
        );
    }
}
