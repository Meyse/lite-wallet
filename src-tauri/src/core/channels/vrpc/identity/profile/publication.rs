//! Backend-owned planning and durable continuation. Quotes never enter the
//! preflight store. Persistence contains intent and receipts, never authority.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::{preflight, read};
use crate::core::auth::session::ActiveWalletAccessContext;
use crate::core::channels::store::PreflightStore;
use crate::core::channels::vrpc::identity::preflight::{parse_funding_utxos, total_satoshis};
use crate::core::channels::vrpc::provider::VrpcProvider;
use crate::core::crypto::{verus_id_signature::extract_chain_height, wif_encoding::Network};
use crate::types::*;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct PublicationBinding {
    pub plan_id: String,
    pub generation: u64,
    pub step: u8,
}

#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct PublicationPlan {
    pub id: String,
    pub network: wallet::WalletNetwork,
    pub request: IdentityProfilePreflightRequest,
    pub generation: u64,
    pub step: u8,
    pub total_steps: u8,
    pub status: ProfilePublicationStatus,
    pub base_revision: String,
    pub header_estimate_sats: Option<String>,
    pub media_digests: std::collections::BTreeMap<String, String>,
    pub first_fee_sats: String,
    pub estimated_total_fee_sats: String,
    pub quote_height: u32,
    pub quote_time: u64,
    pub pending: Option<PendingIdentityProfileUpdate>,
    pub expiry_height: u32,
    pub first_receipt: Option<PendingIdentityProfileUpdate>,
    pub first_previous_revision: Option<String>,
    pub settled_txids: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct PublicationSnapshot {
    pub schema_version: u8,
    pub plans: Vec<PublicationPlan>,
}
impl Default for PublicationSnapshot {
    fn default() -> Self {
        Self {
            schema_version: 1,
            plans: vec![],
        }
    }
}

fn media_digests(
    request: &IdentityProfilePreflightRequest,
) -> Result<std::collections::BTreeMap<String, String>, WalletError> {
    let mut digests = std::collections::BTreeMap::new();
    for (field, media) in [("avatar", &request.avatar), ("header", &request.header)] {
        if let IdentityProfileAvatarChange::Set { value, .. } = media {
            let bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, value)
                .map_err(|_| WalletError::IdentityBuildFailed)?;
            digests.insert(field.into(), hex::encode(Sha256::digest(bytes)));
        }
    }
    Ok(digests)
}

pub(crate) fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

async fn update<T: Send + 'static>(
    access: &ActiveWalletAccessContext,
    mutation: impl FnOnce(&mut PublicationSnapshot) -> Result<T, WalletError> + Send + 'static,
) -> Result<T, WalletError> {
    access
        .stronghold_store
        .update_profile_publications(
            &access.account_id,
            access.password_hash(),
            Some(access.session_submission_guard()),
            mutation,
        )
        .await
}

async fn find(
    access: &ActiveWalletAccessContext,
    identity: &str,
) -> Result<Option<PublicationPlan>, WalletError> {
    let identity = identity.to_string();
    let network = access.wallet_network;
    update(access, move |snapshot| {
        Ok(snapshot
            .plans
            .iter()
            .find(|plan| plan.network == network && plan.request.identity_address == identity)
            .cloned())
    })
    .await
}

fn remaining_request(plan: &PublicationPlan) -> IdentityProfilePreflightRequest {
    let mut request = plan.request.clone();
    if plan.step == 2 {
        request.avatar = IdentityProfileAvatarChange::Keep;
        request.description = IdentityProfileDescriptionChange::Keep;
    }
    request
}

fn public_state(plan: &PublicationPlan) -> ProfilePublicationState {
    ProfilePublicationState {
        plan_id: plan.id.clone(),
        identity_address: plan.request.identity_address.clone(),
        status: plan.status.clone(),
        step: plan.step,
        total_steps: plan.total_steps,
        request: remaining_request(plan),
        pending: plan.pending.clone(),
        completed_receipt: None,
        settled_txids: plan.settled_txids.clone(),
        first_receipt: plan.first_receipt.clone(),
    }
}

fn reconciled_public_state(
    plan: &PublicationPlan,
    receipt_before_reconcile: Option<PendingIdentityProfileUpdate>,
) -> ProfilePublicationState {
    let mut state = public_state(plan);
    if plan.status == ProfilePublicationStatus::Complete {
        // The plan is removed after this response. Preserve its verified receipt
        // so an ambiguous send can still be matched to canonical readback.
        state.completed_receipt = receipt_before_reconcile;
    }
    state
}

struct QuotedPlan {
    first: preflight::PreparedTemplate,
    second: Option<preflight::PreparedTemplate>,
    proposed: IdentityProfileSnapshot,
    fields: Vec<String>,
    groups: Vec<ProfileEvidenceGroup>,
}
impl QuotedPlan {
    fn total(&self) -> Result<i64, WalletError> {
        self.first
            .fee_sats
            .checked_add(self.second.as_ref().map_or(0, |step| step.fee_sats))
            .ok_or(WalletError::IdentityBuildFailed)
    }
    fn steps(&self) -> u8 {
        if self.second.is_some() {
            2
        } else {
            1
        }
    }
}

async fn quote(
    request: &IdentityProfilePreflightRequest,
    context: &preflight::PreparationContext,
    private_key: &[u8; 32],
    provider: &VrpcProvider,
) -> Result<QuotedPlan, WalletError> {
    let complete = preflight::prepare_template(request, context, private_key, provider).await?;
    let proposed = complete.intent.proposed_profile.clone();
    let fields = complete.changed_fields.clone();
    let groups = complete.groups.clone();
    if complete.multipart_images() <= 1 {
        return Ok(QuotedPlan {
            first: complete,
            second: None,
            proposed,
            fields,
            groups,
        });
    }
    let mut first_request = request.clone();
    first_request.header = IdentityProfileAvatarChange::Keep;
    let mut second_request = request.clone();
    second_request.avatar = IdentityProfileAvatarChange::Keep;
    second_request.description = IdentityProfileDescriptionChange::Keep;
    let first = preflight::prepare_template(&first_request, context, private_key, provider).await?;
    let second =
        preflight::prepare_template(&second_request, context, private_key, provider).await?;
    if first.multipart_images() > 1 || second.multipart_images() > 1 {
        return Err(WalletError::IdentityBuildFailed);
    }
    Ok(QuotedPlan {
        first,
        second: Some(second),
        proposed,
        fields,
        groups,
    })
}

pub(crate) async fn prepare(
    mut request: IdentityProfilePreflightRequest,
    resume_id: Option<&str>,
    smaller_field: Option<&str>,
    access: &ActiveWalletAccessContext,
    store: &PreflightStore,
    private_key: &[u8; 32],
    provider: &VrpcProvider,
) -> Result<IdentityProfilePreflightResult, WalletError> {
    let old = find(access, &request.identity_address).await?;
    if let Some(plan) = old.as_ref() {
        if plan.status == ProfilePublicationStatus::Waiting {
            return Err(WalletError::IdentityProfilePublicationPending);
        }
        if plan.step == 2 || plan.status == ProfilePublicationStatus::Stale {
            if resume_id != Some(plan.id.as_str()) {
                return Err(WalletError::IdentityProfilePublicationExists);
            }
            request = remaining_request(plan);
        }
    } else if resume_id.is_some() {
        return Err(WalletError::InvalidPreflight);
    }
    if let Some(field) = smaller_field {
        let plan = old
            .as_ref()
            .filter(|plan| resume_id == Some(plan.id.as_str()))
            .ok_or(WalletError::InvalidPreflight)?;
        let alternative = match field {
            "avatar" if plan.step == 1 => plan.request.smaller_avatar.as_ref(),
            "header" => plan.request.smaller_header.as_ref(),
            _ => None,
        }
        .filter(|image| matches!(image, IdentityProfileAvatarChange::Set { .. }))
        .ok_or(WalletError::InvalidPreflight)?
        .clone();
        if field == "avatar" {
            request.avatar = alternative;
            request.smaller_avatar = None;
        } else {
            request.header = alternative;
            request.smaller_header = None;
        }
    }
    // Optional candidates enter the encrypted snapshot too. Validate their
    // bounds even when their field is unchanged or no useful quote is possible.
    for (candidate, header) in [
        (&request.smaller_avatar, false),
        (&request.smaller_header, true),
    ] {
        if let Some(candidate) = candidate {
            preflight::decode_image_change(candidate, header)?;
        }
    }
    let context = preflight::preparation_context(&request, &access.vrsc_address, provider).await?;
    let mut quoted = quote(&request, &context, private_key, provider).await?;
    let available = total_satoshis(&parse_funding_utxos(
        &provider
            .getaddressutxos(&[access.vrsc_address.clone()])
            .await?,
    ));
    let mut optimization = None;
    // Only offer a quality tradeoff when it removes an approval. Once the
    // remaining changes fit in one update, alternative quotes cannot help.
    for (field, alternative) in [
        ("header", &request.smaller_header),
        ("avatar", &request.smaller_avatar),
    ] {
        if quoted.steps() == 1 {
            break;
        }
        let Some(image @ IdentityProfileAvatarChange::Set { value, .. }) = alternative else {
            continue;
        };
        let original = quoted
            .groups
            .iter()
            .find(|group| group.field == field)
            .map_or(0, |group| group.encoded_bytes);
        let smaller = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, value)
            .map_err(|_| WalletError::IdentityProfileInvalidAvatar)?
            .len();
        if smaller >= original {
            continue;
        }
        let mut alternative_request = request.clone();
        if field == "header" {
            alternative_request.header = image.clone();
        } else {
            alternative_request.avatar = image.clone();
        }
        let alternative_plan = quote(&alternative_request, &context, private_key, provider).await?;
        let saving = quoted.total()? - alternative_plan.total()?;
        if saving > 0 && alternative_plan.steps() < quoted.steps() {
            optimization = Some(ProfileImageOptimization {
                field: field.into(),
                image: image.clone(),
                original_bytes: original,
                smaller_bytes: smaller,
                total_steps: alternative_plan.steps(),
                estimated_total_fee_sats: alternative_plan.total()?.to_string(),
                saving_sats: saving.to_string(),
            });
            break;
        }
    }
    let step = old.as_ref().map_or(1, |plan| plan.step);
    let generation = old.as_ref().map_or(1, |plan| plan.generation + 1);
    let id = old
        .as_ref()
        .map_or_else(|| uuid::Uuid::new_v4().to_string(), |plan| plan.id.clone());
    let mut plan = PublicationPlan {
        id: id.clone(),
        network: access.wallet_network,
        request: request.clone(),
        generation,
        step,
        total_steps: if step == 2 { 2 } else { quoted.steps() },
        status: ProfilePublicationStatus::Ready,
        base_revision: context.revision().into(),
        media_digests: media_digests(&request)?,
        first_fee_sats: quoted.first.fee_sats.to_string(),
        estimated_total_fee_sats: quoted.total()?.to_string(),
        header_estimate_sats: quoted.second.as_ref().map(|step| step.fee_sats.to_string()),
        quote_height: context.height,
        quote_time: now(),
        pending: None,
        expiry_height: 0,
        first_receipt: old.as_ref().and_then(|plan| plan.first_receipt.clone()),
        first_previous_revision: old
            .as_ref()
            .and_then(|plan| plan.first_previous_revision.clone()),
        settled_txids: old
            .as_ref()
            .map_or_else(Vec::new, |plan| plan.settled_txids.clone()),
    };
    if step == 2 {
        let previous = old.as_ref().ok_or(WalletError::InvalidPreflight)?;
        plan.request = previous.request.clone();
        plan.request.header = request.header.clone();
        plan.request.smaller_header = request.smaller_header.clone();
        plan.header_estimate_sats = previous.header_estimate_sats.clone();
        plan.media_digests = media_digests(&plan.request)?;
        plan.first_fee_sats = previous.first_fee_sats.clone();
    }
    let review = ProfilePublicationReview {
        plan_id: Some(id.clone()),
        step,
        total_steps: plan.total_steps,
        next_fee_sats: quoted.second.as_ref().map(|step| step.fee_sats.to_string()),
        estimated_total_fee_sats: quoted.total()?.to_string(),
        earlier_fee_sats: if step == 2 {
            plan.header_estimate_sats.clone()
        } else {
            None
        },
        quote_height: context.height,
        quote_time: plan.quote_time,
        available_sats: available.to_string(),
        proposed_profile: quoted.proposed,
        changed_fields: quoted.fields,
        evidence_groups: quoted.groups,
        optimization,
    };
    quoted.first.intent.publication = Some(PublicationBinding {
        plan_id: id,
        generation,
        step,
    });
    // Only this step is funded and admitted to the session preflight store.
    let result = preflight::finish_preflight(
        quoted.first,
        &request,
        review,
        store,
        &access.account_id,
        &access.session_id,
        &access.vrsc_address,
        &request.channel_id,
        provider,
    )
    .await?;
    let expected = old.map(|plan| (plan.id, plan.generation));
    update(access, move |snapshot| {
        let index = snapshot.plans.iter().position(|existing| {
            existing.network == plan.network
                && existing.request.identity_address == plan.request.identity_address
        });
        if index.map(|index| {
            (
                snapshot.plans[index].id.clone(),
                snapshot.plans[index].generation,
            )
        }) != expected
        {
            return Err(WalletError::InvalidPreflight);
        }
        if let Some(index) = index {
            snapshot.plans[index] = plan;
        } else if snapshot.plans.len() < 100 {
            snapshot.plans.push(plan);
        } else {
            return Err(WalletError::OperationFailed);
        }
        Ok(())
    })
    .await?;
    Ok(result)
}

pub(crate) fn signed_txid(signed_hex: &str) -> Result<String, WalletError> {
    let bytes = hex::decode(signed_hex).map_err(|_| WalletError::InvalidPreflight)?;
    let mut hash = Sha256::digest(Sha256::digest(bytes)).to_vec();
    hash.reverse();
    Ok(hex::encode(hash))
}

// Write the deterministic receipt BEFORE transport. Even a crash or ambiguous RPC
// response can be reconciled without storing or rebroadcasting a signed transaction.
pub(crate) async fn admit_submission(
    access: &ActiveWalletAccessContext,
    binding: &PublicationBinding,
    pending: PendingIdentityProfileUpdate,
    expiry_height: u32,
) -> Result<(), WalletError> {
    let binding = binding.clone();
    let network = access.wallet_network;
    update(access, move |snapshot| {
        let plan = snapshot
            .plans
            .iter_mut()
            .find(|plan| plan.id == binding.plan_id && plan.network == network)
            .ok_or(WalletError::InvalidPreflight)?;
        transition_submission(plan, &binding, pending, expiry_height)?;
        Ok(())
    })
    .await
}

fn transition_submission(
    plan: &mut PublicationPlan,
    binding: &PublicationBinding,
    pending: PendingIdentityProfileUpdate,
    expiry_height: u32,
) -> Result<(), WalletError> {
    if plan.generation != binding.generation
        || plan.step != binding.step
        || plan.status != ProfilePublicationStatus::Ready
        || plan.pending.is_some()
        || plan.request.identity_address != pending.identity_address
    {
        return Err(WalletError::InvalidPreflight);
    }
    plan.status = ProfilePublicationStatus::Waiting;
    plan.pending = Some(pending);
    plan.expiry_height = expiry_height;
    plan.generation += 1;
    Ok(())
}

async fn confirmed(provider: &VrpcProvider, txid: &str) -> Result<bool, WalletError> {
    let tx = provider.getrawtransaction_for_profile(txid, 1).await?;
    if tx.get("txid").and_then(serde_json::Value::as_str) != Some(txid) {
        return Err(WalletError::IdentityProfileUnavailable);
    }
    let confirmations = tx
        .get("confirmations")
        .and_then(serde_json::Value::as_i64)
        .ok_or(WalletError::IdentityProfileUnavailable)?;
    if confirmations <= 0 {
        return Ok(false);
    }
    let Some(blockhash) = tx.get("blockhash").and_then(serde_json::Value::as_str) else {
        return Err(WalletError::IdentityProfileUnavailable);
    };
    let header = provider.getblockheader_for_profile(blockhash).await?;
    if header.get("hash").and_then(serde_json::Value::as_str) != Some(blockhash) {
        return Err(WalletError::IdentityProfileUnavailable);
    }
    Ok(header
        .get("confirmations")
        .and_then(serde_json::Value::as_i64)
        .ok_or(WalletError::IdentityProfileUnavailable)?
        > 0)
}

/// Read-only recovery when another observer already consumed the completed plan.
/// The frontend still matches the returned canonical values against its known receipt.
pub(crate) async fn confirm_profile_revision(
    provider: &VrpcProvider,
    identity: &str,
    txid: &str,
) -> Result<Option<IdentityProfileLoadResult>, WalletError> {
    if txid.len() != 64 || !txid.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(WalletError::IdentityProfileUnavailable);
    }
    let profile = read::load(provider, identity, Network::Testnet).await?;
    if !confirmation_candidate(&profile, txid) || !confirmed(provider, txid).await? {
        return Ok(None);
    }
    Ok(Some(profile))
}

fn confirmation_candidate(profile: &IdentityProfileLoadResult, txid: &str) -> bool {
    profile.state != IdentityProfileState::Unavailable
        && profile.issues.is_empty()
        && profile.revision_txid.as_deref() == Some(txid)
}

pub(crate) async fn reconcile(
    access: &ActiveWalletAccessContext,
    identity: &str,
    provider: &VrpcProvider,
) -> Result<Option<ProfilePublicationState>, WalletError> {
    let Some(mut plan) = find(access, identity).await? else {
        return Ok(None);
    };
    let expected_generation = plan.generation;
    let profile = read::load(provider, identity, Network::Testnet).await?;
    if profile.state == IdentityProfileState::Unavailable || !profile.issues.is_empty() {
        return Err(WalletError::IdentityProfileUnavailable);
    }
    let revision = profile
        .revision_txid
        .as_deref()
        .ok_or(WalletError::IdentityProfileUnavailable)?;
    let revision_confirmed = confirmed(provider, revision).await?;
    let height = extract_chain_height(&provider.getinfo().await?)?;
    let first_confirmed = if let Some(first) = plan.first_receipt.as_ref() {
        Some(
            if plan.first_previous_revision.as_deref() == Some(revision) && revision_confirmed {
                false
            } else {
                confirmed(provider, &first.txid).await?
            },
        )
    } else {
        None
    };
    let receipt_before_reconcile = plan.pending.clone();
    reconcile_observation(
        &mut plan,
        &profile,
        ChainObservation {
            revision_confirmed,
            first_confirmed,
            height,
        },
    )?;
    let state = reconciled_public_state(&plan, receipt_before_reconcile);
    update(access, move |snapshot| {
        let index = snapshot
            .plans
            .iter()
            .position(|existing| existing.id == plan.id)
            .ok_or(WalletError::InvalidPreflight)?;
        if snapshot.plans[index].generation != expected_generation {
            return Err(WalletError::InvalidPreflight);
        }
        if plan.status == ProfilePublicationStatus::Complete {
            snapshot.plans.remove(index);
        } else {
            // Advance only when reconciliation actually changes a persisted state.
            let before = serde_json::to_vec(&snapshot.plans[index])
                .map_err(|_| WalletError::OperationFailed)?;
            if serde_json::to_vec(&plan).map_err(|_| WalletError::OperationFailed)? != before {
                plan.generation += 1;
            }
            snapshot.plans[index] = plan;
        }
        Ok(())
    })
    .await?;
    Ok(Some(state))
}

struct ChainObservation {
    revision_confirmed: bool,
    first_confirmed: Option<bool>,
    height: u32,
}

fn reconcile_observation(
    plan: &mut PublicationPlan,
    profile: &IdentityProfileLoadResult,
    observation: ChainObservation,
) -> Result<(), WalletError> {
    if profile.state == IdentityProfileState::Unavailable || !profile.issues.is_empty() {
        return Err(WalletError::IdentityProfileUnavailable);
    }
    let revision = profile
        .revision_txid
        .as_deref()
        .ok_or(WalletError::IdentityProfileUnavailable)?;
    if let Some(pending) = plan.pending.clone() {
        if revision == pending.txid
            && preflight::current_snapshot(&profile) == pending.proposed_profile
            && observation.revision_confirmed
        {
            plan.settled_txids.push(pending.txid.clone());
            if plan.step == 1 && plan.total_steps == 2 {
                plan.first_previous_revision = Some(plan.base_revision.clone());
                plan.first_receipt = Some(pending);
                plan.step = 2;
                plan.status = ProfilePublicationStatus::Ready;
                plan.base_revision = revision.into();
                plan.pending = None;
            } else {
                plan.status = ProfilePublicationStatus::Complete;
                plan.pending = None;
            }
        } else if revision != plan.base_revision
            && revision != pending.txid
            && observation.revision_confirmed
        {
            plan.status = ProfilePublicationStatus::Stale;
            plan.settled_txids.push(pending.txid.clone());
            plan.pending = None;
        } else if plan.expiry_height > 0
            && observation.height > plan.expiry_height
            && revision == plan.base_revision
            && observation.revision_confirmed
        {
            // The old outpoint is still canonical after expiry: this exact
            // transaction can no longer land. Preserve the draft for fresh review.
            plan.status = ProfilePublicationStatus::Stale;
            plan.settled_txids.push(pending.txid.clone());
            plan.pending = None;
        }
    } else if revision != plan.base_revision && observation.revision_confirmed {
        plan.status = ProfilePublicationStatus::Stale;
    }
    if plan.step == 2 && plan.pending.is_none() {
        if plan.first_receipt.is_some() {
            // A definitive unconfirmed first transaction makes all changes
            // unpublished again. Unavailable RPC evidence must never imply this.
            if observation.first_confirmed == Some(false) {
                plan.step = 1;
                plan.first_receipt = None;
                plan.status = ProfilePublicationStatus::Stale;
            }
        }
    }
    Ok(())
}

pub(crate) async fn discard(
    access: &ActiveWalletAccessContext,
    plan_id: String,
) -> Result<(), WalletError> {
    let network = access.wallet_network;
    update(access, move |snapshot| {
        let index = snapshot
            .plans
            .iter()
            .position(|plan| plan.id == plan_id && plan.network == network)
            .ok_or(WalletError::InvalidPreflight)?;
        if snapshot.plans[index].status == ProfilePublicationStatus::Waiting {
            return Err(WalletError::IdentityProfilePublicationPending);
        }
        snapshot.plans.remove(index);
        Ok(())
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::auth::stronghold_store::StrongholdStore;

    fn snapshot() -> IdentityProfileSnapshot {
        IdentityProfileSnapshot {
            avatar_mime_type: None,
            header_mime_type: None,
            avatar_base64: None,
            header_base64: None,
            avatar_digest: None,
            header_digest: None,
            description: None,
            description_digest: None,
        }
    }
    fn pending() -> PendingIdentityProfileUpdate {
        PendingIdentityProfileUpdate {
            identity_address: "identity".into(),
            txid: "ab".repeat(32),
            submitted_at: 1,
            previous_profile: snapshot(),
            proposed_profile: snapshot(),
        }
    }
    fn plan() -> PublicationPlan {
        PublicationPlan {
            id: "plan".into(),
            network: wallet::WalletNetwork::Testnet,
            request: IdentityProfilePreflightRequest {
                coin_id: "VRSCTEST".into(),
                channel_id: "vrpc.test".into(),
                identity_address: "identity".into(),
                avatar: IdentityProfileAvatarChange::Keep,
                header: IdentityProfileAvatarChange::Set {
                    value: "private-unpublished-header".into(),
                    mime_type: "image/webp".into(),
                },
                description: IdentityProfileDescriptionChange::Set("private description".into()),
                smaller_avatar: None,
                smaller_header: None,
            },
            generation: 1,
            step: 1,
            total_steps: 2,
            status: ProfilePublicationStatus::Ready,
            base_revision: "cd".repeat(32),
            header_estimate_sats: Some("1000000".into()),
            media_digests: Default::default(),
            first_fee_sats: "1000000".into(),
            estimated_total_fee_sats: "2000000".into(),
            quote_height: 100,
            quote_time: 1,
            pending: None,
            expiry_height: 0,
            first_receipt: None,
            first_previous_revision: None,
            settled_txids: vec![],
        }
    }
    #[test]
    fn approvals_are_single_use_bound_to_generation_and_current_step() {
        let binding = PublicationBinding {
            plan_id: "plan".into(),
            generation: 1,
            step: 1,
        };
        let mut plan = plan();
        let mut later = binding.clone();
        later.step = 2;
        assert!(transition_submission(&mut plan, &later, pending(), 120).is_err());
        let mut stale = binding.clone();
        stale.generation = 0;
        assert!(transition_submission(&mut plan, &stale, pending(), 120).is_err());
        transition_submission(&mut plan, &binding, pending(), 120).unwrap();
        assert_eq!(plan.status, ProfilePublicationStatus::Waiting);
        assert_eq!(plan.expiry_height, 120);
        assert!(transition_submission(&mut plan, &binding, pending(), 120).is_err());
        plan.step = 2;
        plan.pending = None;
        plan.status = ProfilePublicationStatus::Ready;
        assert!(transition_submission(&mut plan, &binding, pending(), 120).is_err());
        assert_eq!(
            remaining_request(&plan).avatar,
            IdentityProfileAvatarChange::Keep
        );
        assert_eq!(
            remaining_request(&plan).description,
            IdentityProfileDescriptionChange::Keep
        );
        assert_eq!(remaining_request(&plan).header, plan.request.header);
    }
    #[test]
    fn confirmation_recovery_requires_the_exact_readable_revision_even_for_removal() {
        let mut profile = IdentityProfileLoadResult {
            state: IdentityProfileState::Empty,
            avatar: None,
            header: None,
            description: None,
            issues: vec![],
            read_height: Some(1),
            revision_txid: Some(pending().txid),
        };
        assert!(confirmation_candidate(&profile, &pending().txid));
        assert!(!confirmation_candidate(&profile, &"cd".repeat(32)));
        profile.revision_txid = None;
        assert!(!confirmation_candidate(&profile, &pending().txid));
        profile.revision_txid = Some(pending().txid);
        profile.state = IdentityProfileState::Unavailable;
        assert!(!confirmation_candidate(&profile, &pending().txid));
        // Empty canonical content is only a candidate: confirm_profile_revision
        // still requires the same transaction/block checks as reconciliation.
    }

    #[test]
    fn resumed_progress_exposes_the_first_receipt_without_restoring_authority() {
        let mut plan = plan();
        plan.step = 2;
        plan.first_receipt = Some(pending());
        let state = public_state(&plan);
        assert_eq!(state.first_receipt, Some(pending()));
        assert_eq!(state.request.avatar, IdentityProfileAvatarChange::Keep);
        assert_eq!(
            state.request.description,
            IdentityProfileDescriptionChange::Keep
        );
    }

    #[test]
    fn completed_reconciliation_exposes_the_exact_receipt_once() {
        let mut plan = plan();
        plan.status = ProfilePublicationStatus::Complete;
        plan.pending = None;
        let state = reconciled_public_state(&plan, Some(pending()));
        assert_eq!(state.completed_receipt, Some(pending()));
        assert!(state.pending.is_none());
        plan.status = ProfilePublicationStatus::Ready;
        assert!(reconciled_public_state(&plan, Some(pending()))
            .completed_receipt
            .is_none());
    }

    #[test]
    fn persisted_intent_has_no_preflight_signing_or_session_authority() {
        let serialized = serde_json::to_string(&plan()).unwrap();
        for forbidden in [
            "preflight",
            "signed_hex",
            "session_id",
            "password",
            "private_key",
        ] {
            assert!(!serialized.contains(forbidden));
        }
        let restored: PublicationPlan = serde_json::from_str(&serialized).unwrap();
        assert_eq!(restored.request, plan().request);
        assert_eq!(restored.header_estimate_sats, Some("1000000".into()));
    }

    #[test]
    fn canonical_confirmation_unlocks_review_only_and_reorg_restores_the_full_request() {
        let mut plan = plan();
        plan.status = ProfilePublicationStatus::Waiting;
        plan.pending = Some(pending());
        let original = plan.request.clone();
        let mut profile = IdentityProfileLoadResult {
            state: IdentityProfileState::Empty,
            avatar: None,
            header: None,
            description: None,
            issues: vec![],
            read_height: Some(100),
            revision_txid: Some(pending().txid),
        };
        reconcile_observation(
            &mut plan,
            &profile,
            ChainObservation {
                revision_confirmed: false,
                first_confirmed: None,
                height: 100,
            },
        )
        .unwrap();
        assert_eq!(plan.step, 1);
        assert_eq!(plan.status, ProfilePublicationStatus::Waiting);
        profile.issues.push(IdentityProfileIssue {
            field: None,
            code: "unverified".into(),
        });
        assert!(reconcile_observation(
            &mut plan,
            &profile,
            ChainObservation {
                revision_confirmed: true,
                first_confirmed: None,
                height: 100
            }
        )
        .is_err());
        profile.issues.clear();
        reconcile_observation(
            &mut plan,
            &profile,
            ChainObservation {
                revision_confirmed: true,
                first_confirmed: None,
                height: 100,
            },
        )
        .unwrap();
        assert_eq!(plan.step, 2);
        assert_eq!(plan.status, ProfilePublicationStatus::Ready);
        assert!(plan.pending.is_none());
        assert_eq!(
            remaining_request(&plan).description,
            IdentityProfileDescriptionChange::Keep
        );
        assert_eq!(plan.request, original);
        let mut completed = plan.clone();
        completed.pending = Some(pending());
        completed.status = ProfilePublicationStatus::Waiting;
        reconcile_observation(
            &mut completed,
            &profile,
            ChainObservation {
                revision_confirmed: true,
                first_confirmed: Some(true),
                height: 101,
            },
        )
        .unwrap();
        assert_eq!(completed.status, ProfilePublicationStatus::Complete);
        assert!(completed.pending.is_none());
        profile.revision_txid = Some("ef".repeat(32));
        reconcile_observation(
            &mut plan,
            &profile,
            ChainObservation {
                revision_confirmed: true,
                first_confirmed: Some(false),
                height: 101,
            },
        )
        .unwrap();
        assert_eq!(plan.step, 1);
        assert_eq!(plan.status, ProfilePublicationStatus::Stale);
        assert_eq!(remaining_request(&plan), original);
    }

    #[test]
    fn expiry_or_unrelated_revision_requires_review_and_never_discards_unpublished_bytes() {
        let mut plan = plan();
        plan.status = ProfilePublicationStatus::Waiting;
        plan.pending = Some(pending());
        plan.expiry_height = 120;
        let original = plan.request.clone();
        let mut profile = IdentityProfileLoadResult {
            state: IdentityProfileState::Empty,
            avatar: None,
            header: None,
            description: None,
            issues: vec![],
            read_height: Some(100),
            revision_txid: Some(plan.base_revision.clone()),
        };
        reconcile_observation(
            &mut plan,
            &profile,
            ChainObservation {
                revision_confirmed: true,
                first_confirmed: None,
                height: 119,
            },
        )
        .unwrap();
        assert_eq!(plan.status, ProfilePublicationStatus::Waiting);
        reconcile_observation(
            &mut plan,
            &profile,
            ChainObservation {
                revision_confirmed: true,
                first_confirmed: None,
                height: 121,
            },
        )
        .unwrap();
        assert_eq!(plan.status, ProfilePublicationStatus::Stale);
        assert!(plan.pending.is_none());
        assert_eq!(plan.request, original);
        plan.status = ProfilePublicationStatus::Ready;
        profile.revision_txid = Some("ef".repeat(32));
        reconcile_observation(
            &mut plan,
            &profile,
            ChainObservation {
                revision_confirmed: true,
                first_confirmed: None,
                height: 122,
            },
        )
        .unwrap();
        assert_eq!(plan.status, ProfilePublicationStatus::Stale);
        assert_eq!(plan.request, original);
    }
    #[tokio::test]
    async fn encrypted_continuation_survives_reopen_and_failure_never_erases_it() {
        let _ = iota_stronghold::engine::snapshot::try_set_encrypt_work_factor(0);
        let root =
            std::env::temp_dir().join(format!("profile-publication-{}", uuid::Uuid::new_v4()));
        let store = StrongholdStore::new_for_tests(root.clone());
        let password = [42u8; 32];
        store
            .update_profile_publications("account", &password, None, |snapshot| {
                snapshot.plans.push(plan());
                Ok(())
            })
            .await
            .unwrap();
        let path = root.join("accounts/account/profile_publications.snapshot.stronghold");
        let encrypted = std::fs::read(&path).unwrap();
        assert!(!encrypted
            .windows(b"private-unpublished-header".len())
            .any(|part| part == b"private-unpublished-header"));
        let reopened = StrongholdStore::new_for_tests(root.clone());
        let restored = reopened
            .update_profile_publications("account", &password, None, |snapshot| {
                Ok(snapshot.plans[0].request.clone())
            })
            .await
            .unwrap();
        assert_eq!(restored, plan().request);
        assert!(reopened
            .update_profile_publications("account", &[43u8; 32], None, |snapshot| {
                snapshot.plans.clear();
                Ok(())
            })
            .await
            .is_err());
        assert_eq!(std::fs::read(&path).unwrap(), encrypted);
        assert!(reopened
            .update_profile_publications("account", &password, None, |snapshot| {
                snapshot.plans.clear();
                Err::<(), _>(WalletError::OperationFailed)
            })
            .await
            .is_err());
        assert_eq!(std::fs::read(&path).unwrap(), encrypted);
        reopened
            .update_profile_publications("account", &password, None, |snapshot| {
                snapshot.plans.clear();
                Ok(())
            })
            .await
            .unwrap();
        assert!(reopened
            .update_profile_publications("account", &password, None, |snapshot| Ok(snapshot
                .plans
                .is_empty()))
            .await
            .unwrap());
        std::fs::write(&path, b"corrupt snapshot").unwrap();
        assert!(reopened
            .update_profile_publications("account", &password, None, |_| Ok(()))
            .await
            .is_err());
        assert_eq!(std::fs::read(path).unwrap(), b"corrupt snapshot");
        std::fs::remove_dir_all(root).unwrap();
    }
}
