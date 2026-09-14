//
// Identity update validation and patch application helpers.

use serde_json::Value;

use crate::core::channels::vrpc::provider::VrpcProvider;
use crate::types::{HighRiskChange, IdentityOperation, IdentityPatch, WalletError};

const IDENTITY_FLAG_REVOKED: u64 = 0x8000;

fn get_u64(value: Option<&Value>) -> Option<u64> {
    let v = value?;
    if let Some(x) = v.as_u64() {
        return Some(x);
    }
    if let Some(x) = v.as_i64() {
        return (x >= 0).then_some(x as u64);
    }
    if let Some(x) = v.as_f64() {
        return (x >= 0.0).then_some(x as u64);
    }
    if let Some(x) = v.as_str() {
        if let Ok(parsed) = x.parse::<u64>() {
            return Some(parsed);
        }
    }
    None
}

fn get_string(value: Option<&Value>) -> Option<String> {
    value?.as_str().map(ToString::to_string)
}

fn get_primary_addresses(identity: &Value) -> Vec<String> {
    identity
        .get("primaryaddresses")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|x| x.as_str().map(ToString::to_string))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn is_revoked(identity: &Value) -> bool {
    let flags = get_u64(identity.get("flags")).unwrap_or(0);
    (flags & IDENTITY_FLAG_REVOKED) == IDENTITY_FLAG_REVOKED
}

fn set_revoked(identity: &mut Value, revoked: bool) {
    let current = get_u64(identity.get("flags")).unwrap_or(0);
    let updated = if revoked {
        current | IDENTITY_FLAG_REVOKED
    } else {
        current & !IDENTITY_FLAG_REVOKED
    };
    if let Some(obj) = identity.as_object_mut() {
        obj.insert("flags".to_string(), Value::from(updated));
    }
}

fn apply_patch(identity: &mut Value, patch: Option<&IdentityPatch>) {
    let Some(obj) = identity.as_object_mut() else {
        return;
    };
    let Some(patch) = patch else {
        return;
    };

    if let Some(primary_addresses) = &patch.primary_addresses {
        obj.insert(
            "primaryaddresses".to_string(),
            Value::Array(
                primary_addresses
                    .iter()
                    .map(|x| Value::String(x.clone()))
                    .collect(),
            ),
        );
    }
    if let Some(recovery_authority) = &patch.recovery_authority {
        obj.insert(
            "recoveryauthority".to_string(),
            Value::String(recovery_authority.clone()),
        );
    }
    if let Some(revocation_authority) = &patch.revocation_authority {
        obj.insert(
            "revocationauthority".to_string(),
            Value::String(revocation_authority.clone()),
        );
    }
    if let Some(private_address) = &patch.private_address {
        obj.insert(
            "privateaddress".to_string(),
            Value::String(private_address.clone()),
        );
    }
}

pub fn validate_target_state(
    status: &str,
    operation: &IdentityOperation,
) -> Result<(), WalletError> {
    match operation {
        IdentityOperation::Update | IdentityOperation::Revoke => {
            if status != "active" {
                return Err(WalletError::IdentityInvalidState);
            }
        }
        IdentityOperation::Recover => {
            if status != "revoked" {
                return Err(WalletError::IdentityInvalidState);
            }
        }
    }
    Ok(())
}

pub fn apply_identity_operation(
    identity: &mut Value,
    operation: &IdentityOperation,
    patch: Option<&IdentityPatch>,
) -> Result<(), WalletError> {
    let Some(_) = identity.as_object() else {
        return Err(WalletError::IdentityBuildFailed);
    };

    match operation {
        IdentityOperation::Update => {
            apply_patch(identity, patch);
        }
        IdentityOperation::Revoke => {
            if let Some(obj) = identity.as_object_mut() {
                obj.insert(
                    "contentmultimap".to_string(),
                    Value::Object(Default::default()),
                );
            }
            set_revoked(identity, true);
        }
        IdentityOperation::Recover => {
            if let Some(obj) = identity.as_object_mut() {
                obj.insert(
                    "contentmultimap".to_string(),
                    Value::Object(Default::default()),
                );
            }
            set_revoked(identity, false);
            apply_patch(identity, patch);
        }
    }

    Ok(())
}

fn to_stable_value_string(value: Option<&Value>) -> Option<String> {
    value.and_then(|v| {
        if v.is_null() {
            None
        } else if let Some(s) = v.as_str() {
            Some(s.to_string())
        } else {
            serde_json::to_string(v).ok()
        }
    })
}

pub fn classify_high_risk_changes(before: &Value, after: &Value) -> Vec<HighRiskChange> {
    let mut changes = Vec::new();

    let before_primary = to_stable_value_string(before.get("primaryaddresses"));
    let after_primary = to_stable_value_string(after.get("primaryaddresses"));
    if before_primary != after_primary {
        changes.push(HighRiskChange {
            change_type: "primary_addresses".to_string(),
            before_value: before_primary,
            after_value: after_primary,
        });
    }

    let before_recovery = to_stable_value_string(before.get("recoveryauthority"));
    let after_recovery = to_stable_value_string(after.get("recoveryauthority"));
    if before_recovery != after_recovery {
        changes.push(HighRiskChange {
            change_type: "recovery_authority".to_string(),
            before_value: before_recovery,
            after_value: after_recovery,
        });
    }

    let before_revocation = to_stable_value_string(before.get("revocationauthority"));
    let after_revocation = to_stable_value_string(after.get("revocationauthority"));
    if before_revocation != after_revocation {
        changes.push(HighRiskChange {
            change_type: "revocation_authority".to_string(),
            before_value: before_revocation,
            after_value: after_revocation,
        });
    }

    let revoked_before = is_revoked(before);
    let revoked_after = is_revoked(after);
    if revoked_before != revoked_after {
        changes.push(HighRiskChange {
            change_type: "revoked_state".to_string(),
            before_value: Some(revoked_before.to_string()),
            after_value: Some(revoked_after.to_string()),
        });
    }

    let before_private = to_stable_value_string(before.get("privateaddress"));
    let after_private = to_stable_value_string(after.get("privateaddress"));
    if before_private != after_private {
        changes.push(HighRiskChange {
            change_type: "private_address".to_string(),
            before_value: before_private,
            after_value: after_private,
        });
    }

    changes
}

fn parse_identity_get_result(raw: Value) -> Result<(String, Value), WalletError> {
    let status = get_string(raw.get("status")).ok_or(WalletError::IdentityNotFound)?;
    let identity = raw
        .get("identity")
        .cloned()
        .ok_or(WalletError::IdentityNotFound)?;
    Ok((status, identity))
}

fn map_identity_lookup_error(err: WalletError) -> WalletError {
    match err {
        WalletError::IdentityRpcUnsupported => WalletError::IdentityRpcUnsupported,
        WalletError::NetworkError => WalletError::NetworkError,
        WalletError::IdentityNotFound
        | WalletError::InvalidAddress
        | WalletError::OperationFailed => WalletError::IdentityNotFound,
        _ => WalletError::IdentityNotFound,
    }
}

async fn fetch_identity(
    provider: &VrpcProvider,
    identity_id_or_name: &str,
) -> Result<(String, Value), WalletError> {
    let raw = provider
        .getidentity(identity_id_or_name)
        .await
        .map_err(map_identity_lookup_error)?;
    parse_identity_get_result(raw)
}

fn validate_single_sig_ownership(
    identity: &Value,
    session_address: &str,
) -> Result<(), WalletError> {
    let min_sigs = get_u64(identity.get("minimumsignatures")).unwrap_or(1);
    if min_sigs != 1 {
        return Err(WalletError::IdentityUnsupportedAuthority);
    }

    let primary_addresses = get_primary_addresses(identity);
    if !primary_addresses.iter().any(|a| a == session_address) {
        return Err(WalletError::IdentityUnsupportedAuthority);
    }

    Ok(())
}

pub async fn validate_operation_authority(
    provider: &VrpcProvider,
    operation: &IdentityOperation,
    target_identity: &Value,
    target_status: &str,
    session_address: &str,
) -> Result<(), WalletError> {
    match operation {
        IdentityOperation::Update => {
            if target_status != "active" {
                return Err(WalletError::IdentityInvalidState);
            }
            validate_single_sig_ownership(target_identity, session_address)?;
        }
        IdentityOperation::Revoke => {
            if target_status != "active" {
                return Err(WalletError::IdentityInvalidState);
            }
            let authority = get_string(target_identity.get("revocationauthority"))
                .ok_or(WalletError::IdentityUnsupportedAuthority)?;
            let (status, identity) = fetch_identity(provider, &authority).await?;
            if status != "active" {
                return Err(WalletError::IdentityInvalidState);
            }
            validate_single_sig_ownership(&identity, session_address)?;
        }
        IdentityOperation::Recover => {
            if target_status == "active" {
                return Err(WalletError::IdentityInvalidState);
            }
            let authority = get_string(target_identity.get("recoveryauthority"))
                .ok_or(WalletError::IdentityUnsupportedAuthority)?;
            let (status, identity) = fetch_identity(provider, &authority).await?;
            if status != "active" {
                return Err(WalletError::IdentityInvalidState);
            }
            validate_single_sig_ownership(&identity, session_address)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::channels::vrpc::intent::identity_control_intent_from_json;
    use serde_json::json;

    #[test]
    fn apply_revoke_sets_flag_and_clears_contentmultimap() {
        let mut identity = json!({
            "flags": 0,
            "contentmultimap": { "abc": { "data": "x" } }
        });
        apply_identity_operation(&mut identity, &IdentityOperation::Revoke, None).expect("apply");
        assert_eq!(
            identity["flags"].as_u64().unwrap_or(0) & IDENTITY_FLAG_REVOKED,
            IDENTITY_FLAG_REVOKED
        );
        assert_eq!(identity["contentmultimap"], json!({}));
    }

    #[test]
    fn update_revoke_and_recover_produce_supported_control_intents() {
        let base = json!({
            "version": 3,
            "flags": 0,
            "minimumsignatures": 1,
            "parent": "iJhCezBExJHvtyH3fGhNnt2NhU4Ztkf2yq",
            "name": "operations",
            "contentmultimap": {
                "iK7a5JNJnbeuYWVHCDRpJosj3irGJ5Qa8c": {"message": "preserved update"}
            },
            "contentmap": {},
            "primaryaddresses": [
                "03a058410b33f893fe182f15336577f3941c28c8cadcfb0395b9c31dd5c07ccd11"
            ],
            "revocationauthority": "i98Mnj1YugaRzoURXt4aRhdqQDu7rML9J5",
            "recoveryauthority": "i9ps1xDcr7eM66Fko6aTkeuvvBPZFLEXRN",
            "privateaddresses": [],
            "systemid": "iJhCezBExJHvtyH3fGhNnt2NhU4Ztkf2yq",
            "timelock": 0
        });

        let mut updated = base.clone();
        apply_identity_operation(&mut updated, &IdentityOperation::Update, None).unwrap();
        assert!(identity_control_intent_from_json(&updated).is_ok());

        let mut revoked = base.clone();
        apply_identity_operation(&mut revoked, &IdentityOperation::Revoke, None).unwrap();
        assert!(identity_control_intent_from_json(&revoked).is_ok());

        let mut recovered = revoked;
        apply_identity_operation(&mut recovered, &IdentityOperation::Recover, None).unwrap();
        assert!(identity_control_intent_from_json(&recovered).is_ok());
    }

    #[test]
    fn high_risk_changes_detect_authority_and_revoke_state() {
        let before = json!({
            "primaryaddresses": ["R1"],
            "recoveryauthority": "iBefore",
            "revocationauthority": "iRev1",
            "flags": 0
        });
        let after = json!({
            "primaryaddresses": ["R2"],
            "recoveryauthority": "iAfter",
            "revocationauthority": "iRev2",
            "flags": IDENTITY_FLAG_REVOKED
        });
        let changes = classify_high_risk_changes(&before, &after);
        assert_eq!(changes.len(), 4);
    }

    #[test]
    fn recover_requires_revoked_status() {
        assert!(validate_target_state("revoked", &IdentityOperation::Recover).is_ok());
        assert!(validate_target_state("active", &IdentityOperation::Recover).is_err());
        assert!(validate_target_state("invalid", &IdentityOperation::Recover).is_err());
    }

    #[test]
    fn high_risk_changes_detect_private_address_updates() {
        let before = json!({
            "privateaddress": "zs_before"
        });
        let after = json!({
            "privateaddress": "zs_after"
        });
        let changes = classify_high_risk_changes(&before, &after);
        assert!(changes
            .iter()
            .any(|change| change.change_type == "private_address"));
    }

    #[test]
    fn identity_lookup_error_mapping_preserves_network_error() {
        assert!(matches!(
            map_identity_lookup_error(WalletError::NetworkError),
            WalletError::NetworkError
        ));
    }

    #[test]
    fn identity_lookup_error_mapping_preserves_rpc_unsupported() {
        assert!(matches!(
            map_identity_lookup_error(WalletError::IdentityRpcUnsupported),
            WalletError::IdentityRpcUnsupported
        ));
    }

    #[test]
    fn identity_lookup_error_mapping_maps_lookup_failures_to_not_found() {
        assert!(matches!(
            map_identity_lookup_error(WalletError::OperationFailed),
            WalletError::IdentityNotFound
        ));
        assert!(matches!(
            map_identity_lookup_error(WalletError::InvalidAddress),
            WalletError::IdentityNotFound
        ));
    }
}
