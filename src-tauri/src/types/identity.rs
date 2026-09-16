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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile_update: Option<PendingIdentityProfileUpdate>,
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
    pub system_display_name: Option<String>,
    pub parent: Option<String>,
    pub revocation_authority: Option<String>,
    pub revocation_authority_name: Option<String>,
    pub recovery_authority: Option<String>,
    pub recovery_authority_name: Option<String>,
    pub primary_addresses: Vec<String>,
    pub private_address: Option<String>,
    pub owned_by_primary_address: bool,
    pub minimum_signatures: u32,
    pub tokenized_control: bool,
    pub profile_editable: bool,
    pub profile_editability_reason: Option<String>,
    pub warnings: Vec<IdentityDetailWarning>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IdentityProfileState {
    Ready,
    Empty,
    Unavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IdentityProfileSource {
    pub system_id: String,
    pub txid: String,
    pub vout: u32,
    pub height: u32,
    pub blockhash: String,
    pub digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IdentityProfileAvatar {
    pub base64: String,
    pub mime_type: String,
    pub width: u32,
    pub height: u32,
    pub byte_length: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IdentityProfileField<T> {
    pub value: T,
    pub source: IdentityProfileSource,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IdentityProfileIssue {
    pub field: Option<String>,
    pub code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IdentityProfileLoadResult {
    pub state: IdentityProfileState,
    pub avatar: Option<IdentityProfileField<IdentityProfileAvatar>>,
    pub description: Option<IdentityProfileField<String>>,
    pub issues: Vec<IdentityProfileIssue>,
    pub read_height: Option<u32>,
    pub revision_txid: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "action", content = "value", rename_all = "snake_case")]
pub enum IdentityProfileAvatarChange {
    Keep,
    Set(String),
    Remove,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "action", content = "value", rename_all = "snake_case")]
pub enum IdentityProfileDescriptionChange {
    Keep,
    Set(String),
    Remove,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IdentityProfilePreflightRequest {
    pub coin_id: String,
    pub channel_id: String,
    pub identity_address: String,
    pub avatar: IdentityProfileAvatarChange,
    pub description: IdentityProfileDescriptionChange,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IdentityProfileSnapshot {
    pub avatar_base64: Option<String>,
    pub avatar_digest: Option<String>,
    pub description: Option<String>,
    pub description_digest: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IdentityProfilePreflightResult {
    pub preflight_id: String,
    pub expires_at: u64,
    pub current_profile: IdentityProfileSnapshot,
    pub proposed_profile: IdentityProfileSnapshot,
    pub fee_sats: String,
    pub fee_display: String,
    pub funding_summary: String,
    pub evidence_bytes: usize,
    pub changed_fields: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PendingIdentityProfileUpdate {
    pub identity_address: String,
    pub txid: String,
    pub submitted_at: u64,
    pub previous_profile: IdentityProfileSnapshot,
    pub proposed_profile: IdentityProfileSnapshot,
}
