use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::types::{HighRiskChange, IdentityWarning};

/// A request that passed acceptance and, when signed, signature verification.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenedGenericRequest {
    pub request_hash: String,
    pub signed: bool,
    pub signer_system_id: Option<String>,
    pub signer_identity_id: Option<String>,
    pub signature_block_height: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenericResponseSignerInput {
    pub system_id: String,
    pub identity_id: String,
}

/// An invoice as Rust will pay it.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenericInvoiceReview {
    /// `None` when the invoice accepts any amount.
    pub amount: Option<String>,
    pub currency_id: String,
    pub currency_name: Option<String>,
    pub destination: String,
    pub destination_name: Option<String>,
    pub expiry_height: Option<u64>,
    /// Blocks until the invoice expires, from the chain height at review.
    pub blocks_left: Option<u64>,
    pub already_paid: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenericInvoicePreflightResult {
    pub preflight_id: String,
    pub amount: String,
    pub fee: String,
    pub currency_id: String,
    pub fee_currency_id: String,
    pub from_address: String,
    pub to_address: String,
}

/// The transaction a request's Approval broadcast.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenericRequestTransactionResult {
    pub txid: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildAndSignGenericResponseResult {
    pub signed_response_hex: String,
    pub session_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenericIdentityUpdateRequestMeta {
    pub request_id: Option<String>,
    pub signer_system_id: Option<String>,
    pub signer_identity_id: Option<String>,
    pub expiry_height: Option<u32>,
    pub request_system_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenericIdentityPrimaryAddressEntry {
    pub address: String,
    pub in_wallet: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenericIdentityPrimaryAddressInfo {
    pub addresses: Vec<GenericIdentityPrimaryAddressEntry>,
    pub wallet_count: usize,
    pub external_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenericIdentityAuthorities {
    pub revocation: Option<String>,
    pub recovery: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenericIdentityUpdateReviewResult {
    pub target_identity: String,
    pub warnings: Vec<IdentityWarning>,
    pub high_risk_changes: Vec<HighRiskChange>,
    pub current_identity: Value,
    pub requested_identity: Value,
    pub fully_qualified_name: Option<String>,
    pub friendly_names: HashMap<String, String>,
    pub signer_cmm_key_labels: HashMap<String, String>,
    pub primary_address_after_update_info: GenericIdentityPrimaryAddressInfo,
    pub current_authorities: GenericIdentityAuthorities,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenericIdentityUpdatePreflightResult {
    pub preflight_id: String,
    pub target_identity: String,
    pub from_address: String,
    pub fee: String,
    pub fee_currency: String,
    pub warnings: Vec<IdentityWarning>,
    pub high_risk_changes: Vec<HighRiskChange>,
    pub current_identity: Value,
    pub requested_identity: Value,
    pub fully_qualified_name: Option<String>,
    pub friendly_names: HashMap<String, String>,
    pub signer_cmm_key_labels: HashMap<String, String>,
    pub primary_address_after_update_info: GenericIdentityPrimaryAddressInfo,
    pub current_authorities: GenericIdentityAuthorities,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProvisioningJobRecord {
    pub job_id: String,
    pub request_type: String,
    pub request_hex: String,
    pub requested_identity_address: Option<String>,
    pub requested_fqn: String,
    pub signing_id: String,
    pub has_response_uris: bool,
    pub info_uri: Option<String>,
    pub status: String,
    pub created_at: u64,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkReadyProvisioningJobResult {
    pub job: ProvisioningJobRecord,
    pub linked_identities: Vec<crate::types::LinkedIdentity>,
}
