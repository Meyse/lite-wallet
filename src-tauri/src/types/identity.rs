//
// Identity update command types.
// Security: backend controls tx building/signing; UI sends only preflight_id for send.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IdentityOperation {
    Update,
    Revoke,
    Recover,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct IdentityPatch {
    pub primary_addresses: Option<Vec<String>>,
    pub recovery_authority: Option<String>,
    pub revocation_authority: Option<String>,
    pub private_address: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HighRiskChange {
    pub change_type: String,
    pub before_value: Option<String>,
    pub after_value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IdentityWarning {
    pub warning_type: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IdentityPreflightParams {
    pub coin_id: String,
    pub channel_id: String,
    pub operation: IdentityOperation,
    pub target_identity: String,
    pub patch: Option<IdentityPatch>,
    pub memo: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IdentityPreflightResult {
    pub preflight_id: String,
    pub operation: IdentityOperation,
    pub target_identity: String,
    pub from_address: String,
    pub fee: String,
    pub fee_currency: String,
    pub high_risk_changes: Vec<HighRiskChange>,
    pub warnings: Vec<IdentityWarning>,
    pub memo: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IdentitySendRequest {
    pub preflight_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IdentitySendResult {
    pub txid: String,
    pub operation: IdentityOperation,
    pub target_identity: String,
    pub fee: String,
    pub from_address: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LinkableIdentity {
    pub identity_address: String,
    pub name: Option<String>,
    pub fully_qualified_name: Option<String>,
    pub status: Option<String>,
    pub linked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LinkedIdentity {
    pub identity_address: String,
    pub name: Option<String>,
    pub fully_qualified_name: Option<String>,
    pub status: Option<String>,
    pub system_id: Option<String>,
    #[serde(default)]
    pub favorite: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkIdentityRequest {
    pub identity_address: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnlinkIdentityRequest {
    pub identity_address: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetLinkedIdentityFavoriteRequest {
    pub identity_address: String,
    pub favorite: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IdentityDetailWarning {
    pub warning_type: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IdentityDetails {
    pub identity_address: String,
    pub name: Option<String>,
    pub fully_qualified_name: Option<String>,
    pub status: Option<String>,
    pub system: Option<String>,
    pub parent: Option<String>,
    pub revocation_authority: Option<String>,
    pub recovery_authority: Option<String>,
    pub primary_addresses: Vec<String>,
    pub private_address: Option<String>,
    pub owned_by_primary_address: bool,
    pub warnings: Vec<IdentityDetailWarning>,
}
