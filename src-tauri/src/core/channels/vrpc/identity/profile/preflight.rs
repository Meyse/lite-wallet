use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine as _;
use blake2b_simd::Params as Blake2bParams;
use rand::{rngs::OsRng, RngCore};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use unicode_segmentation::UnicodeSegmentation;
use uuid::Uuid;

use crate::core::channels::store::{PreflightRecord, PreflightStore};
use crate::core::channels::vrpc::common::{authenticate_payload_inputs, VrpcInputRef};
use crate::core::channels::vrpc::identity::preflight::{
    build_unsigned_identity_tx, fetch_identity_prevout, parse_funding_utxos, parse_target_identity,
    parse_updateidentity_hex, sat_to_decimal_string, total_satoshis, IdentityPreflightPayload,
    TargetIdentityState, IDENTITY_PREFLIGHT_TTL,
};
use crate::core::channels::vrpc::identity::profile::codec::{
    descriptor_from_json, resolve_profile_payload, validate_avatar_bytes, validate_header_bytes,
    DataDescriptor, AVATAR_MIME, AVATAR_VDXF_KEY, DESCRIPTION_MIME, DESCRIPTION_VDXF_KEY,
    HEADER_VDXF_KEY, MAX_AVATAR_BYTES, MAX_DESCRIPTION_BYTES, MAX_HEADER_BYTES, REMOVE_VDXF_KEY,
};
use crate::core::channels::vrpc::identity::profile::intent::{
    validate as validate_profile_intent, ProfileTransactionIntent,
};
use crate::core::channels::vrpc::identity::profile::read::{active_values, load};
use crate::core::channels::vrpc::identity::verus_tx::codec::decode_hex as decode_verus_tx;
use crate::core::channels::vrpc::intent::{
    decode_base58_destination, identity_control_intent_from_json, normalize_vdxf_univalue,
    validate_identity_control_intent,
};
use crate::core::channels::vrpc::provider::VrpcProvider;
use crate::core::crypto::verus_id_signature::{
    compute_verus_data_identity_hash, extract_chain_height, sign_identity_hash_with_private_bytes,
};
use crate::core::crypto::wif_encoding::Network;
use crate::types::{
    IdentityOperation, IdentityProfileAvatarChange, IdentityProfileDescriptionChange,
    IdentityProfileLoadResult, IdentityProfilePreflightRequest, IdentityProfilePreflightResult,
    IdentityProfileSnapshot, IdentityProfileState, ProfileEvidenceGroup, ProfilePublicationReview,
    WalletError,
};

const VRSCTEST_SYSTEM_ID: &str = "iJhCezBExJHvtyH3fGhNnt2NhU4Ztkf2yq";
const DEFAULT_TRANSACTION_FEE: i64 = 10_000;
const MAX_SCRIPT_ELEMENT_SIZE: usize = 6_000;
const STORAGE_FEE_FACTOR: i64 = 6;
const MAX_DESCRIPTION_GRAPHEMES: usize = 160;
const REMOVE_HASH_PERSONALIZATION: &[u8; 16] = b"VerusDefaultHash";

fn non_empty_string(value: Option<&Value>) -> Option<String> {
    let value = value?.as_str()?.trim();
    (!value.is_empty()).then(|| value.to_string())
}

pub(crate) fn current_snapshot(profile: &IdentityProfileLoadResult) -> IdentityProfileSnapshot {
    IdentityProfileSnapshot {
        avatar_mime_type: profile
            .avatar
            .as_ref()
            .map(|field| field.value.mime_type.clone()),
        header_mime_type: profile
            .header
            .as_ref()
            .map(|field| field.value.mime_type.clone()),
        header_base64: profile
            .header
            .as_ref()
            .map(|field| field.value.base64.clone()),
        header_digest: profile
            .header
            .as_ref()
            .map(|field| field.source.digest.clone()),
        avatar_base64: profile
            .avatar
            .as_ref()
            .map(|field| field.value.base64.clone()),
        avatar_digest: profile
            .avatar
            .as_ref()
            .map(|field| field.source.digest.clone()),
        description: profile
            .description
            .as_ref()
            .map(|field| field.value.clone()),
        description_digest: profile
            .description
            .as_ref()
            .map(|field| field.source.digest.clone()),
    }
}

fn normalize_description(value: &str) -> Result<String, WalletError> {
    let normalized = value.replace("\r\n", "\n").replace('\r', "\n");
    let normalized = normalized.trim().to_string();
    if normalized.len() > MAX_DESCRIPTION_BYTES
        || normalized.graphemes(true).count() > MAX_DESCRIPTION_GRAPHEMES
    {
        return Err(WalletError::IdentityProfileInvalidDescription);
    }
    Ok(normalized)
}

fn decode_identity_id(value: &str) -> Result<[u8; 20], WalletError> {
    decode_base58_destination(value)?
        .destination_bytes
        .try_into()
        .map_err(|_| WalletError::IdentityBuildFailed)
}

fn signed_data_value(
    identity_address: &str,
    system_id: &str,
    bytes: &[u8],
    mime_type: &str,
    label: &str,
    height: u32,
    private_key: &[u8; 32],
) -> Result<(Value, String), WalletError> {
    let mut salt = [0u8; 32];
    OsRng.fill_bytes(&mut salt);
    let mut digest_input = bytes.to_vec();
    digest_input.extend_from_slice(&salt);
    let digest: [u8; 32] = Sha256::digest(&digest_input).into();
    let identity_hash = compute_verus_data_identity_hash(
        decode_identity_id(system_id)?,
        decode_identity_id(identity_address)?,
        height,
        digest,
    );
    let signature = sign_identity_hash_with_private_bytes(identity_hash, height, private_key)
        .map_err(|_| WalletError::IdentitySignFailed)?;
    Ok((
        json!({
            "data": {
                "address": identity_address,
                // A direct `messagehex` wrapper makes verusd generate a fresh
                // salt even when `mmrsalt` is present. Supplying one explicit
                // MMR item is the supported path that preserves the exact salt
                // covered by the locally produced identity signature.
                "mmrdata": {
                    "serializedhex": hex::encode(bytes),
                    "label": label,
                    "mimetype": mime_type
                },
                "mmrsalt": hex::encode(salt),
                "hashtype": "sha256",
                "createmmr": true,
                "signature": BASE64_STANDARD.encode(signature),
                "label": label,
                "mimetype": mime_type
            }
        }),
        hex::encode(digest),
    ))
}

fn exact_value_hash(value: &Value) -> Result<String, WalletError> {
    let serialized = normalize_vdxf_univalue(value)?;
    let mut digest = Blake2bParams::new()
        .hash_length(32)
        .personal(REMOVE_HASH_PERSONALIZATION)
        .hash(&serialized)
        .as_bytes()
        .to_vec();
    digest.reverse();
    Ok(hex::encode(digest))
}

fn removal_value(key: &str, value: &Value) -> Result<Value, WalletError> {
    Ok(json!({
        REMOVE_VDXF_KEY: {
            "version": 1,
            "action": 2,
            "entrykey": key,
            "valuehash": exact_value_hash(value)?
        }
    }))
}

fn identity_primary(decoded: &Value) -> Result<Value, WalletError> {
    let outputs = decoded
        .get("vout")
        .and_then(Value::as_array)
        .ok_or(WalletError::IdentityBuildFailed)?;
    let mut matches = outputs.iter().filter_map(|output| {
        output
            .pointer("/scriptPubKey/identityprimary")
            .filter(|value| value.is_object())
            .cloned()
    });
    let identity = matches.next().ok_or(WalletError::IdentityBuildFailed)?;
    if matches.next().is_some() {
        return Err(WalletError::IdentityBuildFailed);
    }
    Ok(identity)
}

fn generated_descriptors(
    map: &serde_json::Map<String, Value>,
    key: &str,
) -> Result<Vec<DataDescriptor>, WalletError> {
    let values = map
        .get(key)
        .and_then(Value::as_array)
        .filter(|values| values.len() == 1)
        .ok_or(WalletError::IdentityBuildFailed)?;
    let group = values[0]
        .as_array()
        .filter(|group| group.len() == 2)
        .ok_or(WalletError::IdentityBuildFailed)?;
    group
        .iter()
        .map(|entry| {
            descriptor_from_json(
                entry
                    .get("i4GC1YGEVD21afWudGoFJVdnfjJ5XWnCQv")
                    .ok_or(WalletError::IdentityBuildFailed)?,
            )
            .map_err(|_| WalletError::IdentityBuildFailed)
        })
        .collect()
}

fn compact_size_len(value: usize) -> usize {
    if value < 253 {
        1
    } else if value <= u16::MAX as usize {
        3
    } else if value <= u32::MAX as usize {
        5
    } else {
        9
    }
}

fn content_multimap_size(
    intent: &crate::core::channels::vrpc::intent::IdentityControlIntent,
) -> usize {
    let mut size = compact_size_len(intent.content_multimap.len());
    for (_, values) in &intent.content_multimap {
        size += 20 + compact_size_len(values.len());
        for value in values {
            size += compact_size_len(value.len()) + value.len();
        }
    }
    size.saturating_sub(1)
}

fn decimal_coin_to_sats(value: &str) -> Result<i64, WalletError> {
    let value = value.trim();
    if value.is_empty() || value.starts_with('-') || value.starts_with('+') {
        return Err(WalletError::IdentityBuildFailed);
    }
    let mut parts = value.split('.');
    let whole = parts.next().ok_or(WalletError::IdentityBuildFailed)?;
    let fractional = parts.next().unwrap_or("");
    if parts.next().is_some()
        || whole.is_empty()
        || !whole.bytes().all(|byte| byte.is_ascii_digit())
        || !fractional.bytes().all(|byte| byte.is_ascii_digit())
        || fractional.len() > 8
    {
        return Err(WalletError::IdentityBuildFailed);
    }
    let whole = whole
        .parse::<i64>()
        .map_err(|_| WalletError::IdentityBuildFailed)?;
    let fractional = if fractional.is_empty() {
        0
    } else {
        fractional
            .parse::<i64>()
            .map_err(|_| WalletError::IdentityBuildFailed)?
            .checked_mul(10i64.pow(8 - fractional.len() as u32))
            .ok_or(WalletError::IdentityBuildFailed)?
    };
    whole
        .checked_mul(100_000_000)
        .and_then(|sats| sats.checked_add(fractional))
        .filter(|sats| *sats > 0)
        .ok_or(WalletError::IdentityBuildFailed)
}

fn transaction_export_fee_sats(currency: &Value) -> Result<i64, WalletError> {
    let value = currency
        .get("transactionexportfee")
        .ok_or(WalletError::IdentityBuildFailed)?;
    let serialized;
    let value = if let Some(value) = value.as_str() {
        value
    } else if value.is_number() {
        serialized = value.to_string();
        &serialized
    } else {
        return Err(WalletError::IdentityBuildFailed);
    };
    decimal_coin_to_sats(value)
}

fn calculate_fee(
    template: &crate::core::channels::vrpc::identity::verus_tx::model::VerusTx,
    decoded: &Value,
    control: &crate::core::channels::vrpc::intent::IdentityControlIntent,
    export_fee_sats: i64,
) -> Result<(i64, usize), WalletError> {
    let identity_bytes = content_multimap_size(control);
    let identity_factor = (identity_bytes + 127) / 128
        + control.content_map.len()
        + control.primary_addresses.len().saturating_sub(1)
        + control.private_addresses.len().saturating_sub(1);
    let mut fee = i64::try_from(identity_factor)
        .ok()
        .and_then(|factor| factor.checked_mul(DEFAULT_TRANSACTION_FEE))
        .ok_or(WalletError::IdentityBuildFailed)?;
    if fee == 0 {
        fee = DEFAULT_TRANSACTION_FEE;
    }
    if template.outputs.len() > 1 {
        fee = fee
            .checked_add(
                i64::try_from(template.outputs.len() - 1)
                    .map_err(|_| WalletError::IdentityBuildFailed)?
                    .checked_mul(DEFAULT_TRANSACTION_FEE)
                    .ok_or(WalletError::IdentityBuildFailed)?,
            )
            .ok_or(WalletError::IdentityBuildFailed)?;
    }

    let threshold = MAX_SCRIPT_ELEMENT_SIZE / 3;
    for output in &template.outputs {
        let extra = output.script_pub_key.len().saturating_sub(threshold);
        if extra > 0 {
            fee = fee
                .checked_add(DEFAULT_TRANSACTION_FEE)
                .and_then(|value| {
                    if extra > threshold {
                        value.checked_add(DEFAULT_TRANSACTION_FEE)
                    } else {
                        Some(value)
                    }
                })
                .ok_or(WalletError::IdentityBuildFailed)?;
        }
    }

    let decoded_outputs = decoded
        .get("vout")
        .and_then(Value::as_array)
        .ok_or(WalletError::IdentityBuildFailed)?;
    if decoded_outputs.len() != template.outputs.len() {
        return Err(WalletError::IdentityBuildFailed);
    }
    let mut storage_started = false;
    let mut storage_bytes = 0usize;
    for (index, output) in template.outputs.iter().enumerate() {
        if decoded_outputs[index]
            .pointer("/scriptPubKey/notaryevidence")
            .is_some()
        {
            storage_started = true;
        }
        if storage_started {
            storage_bytes = storage_bytes
                .checked_add(output.script_pub_key.len())
                .ok_or(WalletError::IdentityBuildFailed)?;
        } else {
            let extra = output.script_pub_key.len().saturating_sub(threshold);
            if extra > 0 {
                fee = fee
                    .checked_add(DEFAULT_TRANSACTION_FEE)
                    .and_then(|value| {
                        if extra > threshold {
                            value.checked_add(DEFAULT_TRANSACTION_FEE)
                        } else {
                            Some(value)
                        }
                    })
                    .ok_or(WalletError::IdentityBuildFailed)?;
            }
        }
    }
    let storage_fee = i64::try_from(storage_bytes)
        .ok()
        .and_then(|bytes| bytes.checked_mul(STORAGE_FEE_FACTOR))
        .and_then(|value| value.checked_mul(export_fee_sats))
        .map(|value| value / MAX_SCRIPT_ELEMENT_SIZE as i64)
        .ok_or(WalletError::IdentityBuildFailed)?;
    Ok((
        fee.checked_add(storage_fee)
            .ok_or(WalletError::IdentityBuildFailed)?,
        storage_bytes,
    ))
}

pub(crate) fn decode_image_change(
    change: &IdentityProfileAvatarChange,
    header: bool,
) -> Result<Option<Vec<u8>>, WalletError> {
    let IdentityProfileAvatarChange::Set { value, mime_type } = change else {
        return Ok(None);
    };
    let invalid = || {
        if header {
            WalletError::IdentityProfileInvalidHeader
        } else {
            WalletError::IdentityProfileInvalidAvatar
        }
    };
    let maximum = if header {
        MAX_HEADER_BYTES
    } else {
        MAX_AVATAR_BYTES
    };
    if value.len() > maximum.div_ceil(3) * 4 {
        return Err(invalid());
    }
    if mime_type != AVATAR_MIME {
        return Err(invalid());
    }
    let bytes = BASE64_STANDARD.decode(value).map_err(|_| invalid())?;
    if header {
        validate_header_bytes(&bytes, mime_type)?;
    } else {
        validate_avatar_bytes(&bytes, mime_type)?;
    }
    Ok(Some(bytes))
}

fn image_changes(
    change: &IdentityProfileAvatarChange,
    bytes: Option<&[u8]>,
    current: Option<&str>,
) -> bool {
    match (change, bytes, current) {
        (IdentityProfileAvatarChange::Keep, _, _) => false,
        (IdentityProfileAvatarChange::Remove, _, current) => current.is_some(),
        (IdentityProfileAvatarChange::Set { .. }, Some(bytes), Some(current)) => BASE64_STANDARD
            .decode(current)
            .map(|value| value != bytes)
            .unwrap_or(true),
        (IdentityProfileAvatarChange::Set { .. }, Some(_), None) => true,
        _ => false,
    }
}

pub(crate) struct PreparationContext {
    target: TargetIdentityState,
    current: IdentityProfileSnapshot,
    avatar_values: Vec<Value>,
    header_values: Vec<Value>,
    description_values: Vec<Value>,
    system_id: String,
    primary_addresses: Vec<String>,
    pub height: u32,
    export_fee_sats: i64,
}
impl PreparationContext {
    pub fn revision(&self) -> &str {
        &self.target.txid
    }
}

pub(crate) struct PreparedTemplate {
    pub tx: crate::core::channels::vrpc::identity::verus_tx::model::VerusTx,
    control: crate::core::channels::vrpc::intent::IdentityControlIntent,
    pub intent: ProfileTransactionIntent,
    pub fee_sats: i64,
    pub evidence_bytes: usize,
    pub groups: Vec<ProfileEvidenceGroup>,
    pub changed_fields: Vec<String>,
}
impl PreparedTemplate {
    pub fn multipart_images(&self) -> usize {
        self.groups
            .iter()
            .filter(|group| group.parts > 1 && (group.field == "avatar" || group.field == "header"))
            .count()
    }
}

pub(crate) async fn preparation_context(
    request: &IdentityProfilePreflightRequest,
    from_address: &str,
    provider: &VrpcProvider,
) -> Result<PreparationContext, WalletError> {
    if request.coin_id != "VRSCTEST" || request.identity_address.trim().is_empty() {
        return Err(WalletError::IdentityProfileWriteUnsupported);
    }
    let target = parse_target_identity(
        provider
            .getidentity_for_profile(&request.identity_address)
            .await?,
    )?;
    if !target.status.eq_ignore_ascii_case("active") {
        return Err(WalletError::IdentityProfileReadOnly);
    }
    let system_id = non_empty_string(
        target
            .identity
            .get("systemid")
            .or_else(|| target.identity.get("systemId")),
    )
    .ok_or(WalletError::IdentityProfileReadOnly)?;
    let minimum_signatures = target
        .identity
        .get("minimumsignatures")
        .or_else(|| target.identity.get("minimumSignatures"))
        .and_then(Value::as_u64)
        .unwrap_or_default();
    let flags = target
        .identity
        .get("flags")
        .and_then(Value::as_u64)
        .unwrap_or_default();
    let primary_addresses = target
        .identity
        .get("primaryaddresses")
        .or_else(|| target.identity.get("primaryAddresses"))
        .and_then(Value::as_array)
        .ok_or(WalletError::IdentityProfileReadOnly)?
        .iter()
        .filter_map(|value| non_empty_string(Some(value)))
        .collect::<Vec<_>>();
    if system_id != VRSCTEST_SYSTEM_ID
        || minimum_signatures != 1
        || flags & 0x4 != 0
        || primary_addresses.len() != 1
        || !primary_addresses[0].eq_ignore_ascii_case(from_address)
    {
        return Err(WalletError::IdentityProfileReadOnly);
    }

    let current_profile = load(provider, &request.identity_address, Network::Testnet).await?;
    if matches!(current_profile.state, IdentityProfileState::Unavailable)
        || !current_profile.issues.is_empty()
    {
        return Err(WalletError::IdentityProfileUnavailable);
    }
    let current = current_snapshot(&current_profile);
    let avatar_values = active_values(
        provider,
        &request.identity_address,
        AVATAR_VDXF_KEY,
        &target.txid,
    )
    .await?;
    let header_values = active_values(
        provider,
        &request.identity_address,
        HEADER_VDXF_KEY,
        &target.txid,
    )
    .await?;
    let description_values = active_values(
        provider,
        &request.identity_address,
        DESCRIPTION_VDXF_KEY,
        &target.txid,
    )
    .await?;
    if avatar_values.len() > 1 || header_values.len() > 1 || description_values.len() > 1 {
        return Err(WalletError::IdentityProfileUnavailable);
    }

    if current_profile.revision_txid.as_deref() != Some(target.txid.as_str()) {
        return Err(WalletError::InvalidPreflight);
    }
    let height = extract_chain_height(&provider.getinfo().await?)
        .map_err(|_| WalletError::IdentityBuildFailed)?;
    let export_fee_sats = transaction_export_fee_sats(&provider.getcurrency(&system_id).await?)?;
    Ok(PreparationContext {
        target,
        current,
        avatar_values,
        header_values,
        description_values,
        system_id,
        primary_addresses,
        height,
        export_fee_sats,
    })
}

pub(crate) async fn prepare_template(
    request: &IdentityProfilePreflightRequest,
    context: &PreparationContext,
    private_key: &[u8; 32],
    provider: &VrpcProvider,
) -> Result<PreparedTemplate, WalletError> {
    let PreparationContext {
        target,
        current,
        avatar_values,
        header_values,
        description_values,
        system_id,
        primary_addresses,
        height,
        export_fee_sats,
    } = context;
    let height = *height;
    let avatar_bytes = decode_image_change(&request.avatar, false)?;
    let header_bytes = decode_image_change(&request.header, true)?;
    let normalized_description = match &request.description {
        IdentityProfileDescriptionChange::Keep => None,
        IdentityProfileDescriptionChange::Set(value) => Some(normalize_description(value)?),
        IdentityProfileDescriptionChange::Remove => Some(String::new()),
    };
    let avatar_changes = image_changes(
        &request.avatar,
        avatar_bytes.as_deref(),
        current.avatar_base64.as_deref(),
    );
    let header_changes = image_changes(
        &request.header,
        header_bytes.as_deref(),
        current.header_base64.as_deref(),
    );
    let description_changes = match &normalized_description {
        None => false,
        Some(value) if value.is_empty() => current.description.is_some(),
        Some(value) => current.description.as_deref() != Some(value.as_str()),
    };
    if !avatar_changes && !header_changes && !description_changes {
        return Err(WalletError::IdentityProfileNoChanges);
    }

    let mut content = BTreeMap::<String, Vec<Value>>::new();
    let mut removals = Vec::new();
    let mut proposed = current.clone();
    let mut avatar_digest = None;
    let mut header_digest = None;
    let mut description_digest = None;
    let mut changed_fields = Vec::new();
    for (changes, key, label, values, bytes, digest_slot, base64_slot, snapshot_digest) in [
        (
            avatar_changes,
            AVATAR_VDXF_KEY,
            "avatar",
            &avatar_values,
            &avatar_bytes,
            &mut avatar_digest,
            &mut proposed.avatar_base64,
            &mut proposed.avatar_digest,
        ),
        (
            header_changes,
            HEADER_VDXF_KEY,
            "header",
            &header_values,
            &header_bytes,
            &mut header_digest,
            &mut proposed.header_base64,
            &mut proposed.header_digest,
        ),
    ] {
        if !changes {
            continue;
        }
        changed_fields.push(label.to_string());
        if let Some(value) = values.first() {
            removals.push(removal_value(key, value)?);
        }
        *base64_slot = None;
        *snapshot_digest = None;
        if let Some(bytes) = bytes.as_ref() {
            let (value, digest) = signed_data_value(
                &request.identity_address,
                &system_id,
                bytes,
                AVATAR_MIME,
                label,
                height,
                private_key,
            )?;
            content.insert(key.to_string(), vec![value]);
            *base64_slot = Some(BASE64_STANDARD.encode(bytes));
            *snapshot_digest = Some(digest.clone());
            *digest_slot = Some(digest);
        }
    }
    if avatar_changes {
        proposed.avatar_mime_type = avatar_bytes.as_ref().map(|_| AVATAR_MIME.to_string());
    }
    if header_changes {
        proposed.header_mime_type = header_bytes.as_ref().map(|_| AVATAR_MIME.to_string());
    }
    if description_changes {
        proposed.description = normalized_description
            .clone()
            .filter(|value| !value.is_empty());
        proposed.description_digest = None;
        changed_fields.push("description".to_string());
        if let Some(value) = description_values.first() {
            removals.push(removal_value(DESCRIPTION_VDXF_KEY, value)?);
        }
        if let Some(value) = normalized_description
            .as_ref()
            .filter(|value| !value.is_empty())
        {
            let (data, digest) = signed_data_value(
                &request.identity_address,
                &system_id,
                value.as_bytes(),
                DESCRIPTION_MIME,
                "description",
                height,
                private_key,
            )?;
            content.insert(DESCRIPTION_VDXF_KEY.to_string(), vec![data]);
            proposed.description_digest = Some(digest.clone());
            description_digest = Some(digest);
        }
    }
    if !removals.is_empty() {
        content.insert(REMOVE_VDXF_KEY.to_string(), removals.clone());
    }

    for (key, digest) in [
        (AVATAR_VDXF_KEY, avatar_digest.as_ref()),
        (HEADER_VDXF_KEY, header_digest.as_ref()),
        (DESCRIPTION_VDXF_KEY, description_digest.as_ref()),
    ] {
        let Some(digest) = digest else { continue };
        let signature = content
            .get(key)
            .and_then(|values| values.first())
            .and_then(|value| value.pointer("/data/signature"))
            .and_then(Value::as_str)
            .ok_or(WalletError::IdentitySignFailed)?;
        let verification = provider
            .verifysignature(&json!({
                "address": request.identity_address,
                "datahash": digest,
                "hashtype": "sha256",
                "signature": signature
            }))
            .await?;
        if verification.get("signaturestatus").and_then(Value::as_str) != Some("verified")
            || verification.get("address").and_then(Value::as_str)
                != Some(request.identity_address.as_str())
            || verification.get("systemid").and_then(Value::as_str) != Some(system_id.as_str())
            || verification.get("hash").and_then(Value::as_str) != Some(digest.as_str())
            || verification.get("signatureheight").and_then(Value::as_u64)
                != Some(u64::from(height))
        {
            return Err(WalletError::IdentitySignFailed);
        }
    }

    let mut update_identity = target.identity.clone();
    update_identity["contentmultimap"] =
        serde_json::to_value(&content).map_err(|_| WalletError::IdentityBuildFailed)?;
    let template_hex = parse_updateidentity_hex(
        provider
            .updateidentity_with_options(&update_identity, true, false, None, None)
            .await?,
    )?;
    let decoded = provider.decoderawtransaction(&template_hex).await?;
    let decoded_identity = identity_primary(&decoded)?;
    let mut target_controls = identity_control_intent_from_json(&target.identity)?;
    let actual_control = identity_control_intent_from_json(&decoded_identity)?;
    let mut actual_controls_only = actual_control.clone();
    target_controls.content_multimap.clear();
    actual_controls_only.content_multimap.clear();
    if target_controls != actual_controls_only {
        return Err(WalletError::IdentityBuildFailed);
    }
    validate_identity_control_intent(&template_hex, &actual_control)?;
    let actual_map = decoded_identity
        .get("contentmultimap")
        .and_then(Value::as_object)
        .ok_or(WalletError::IdentityBuildFailed)?;
    if actual_map.len() != content.len()
        || content
            .keys()
            .any(|key| !actual_map.contains_key(key.as_str()))
    {
        return Err(WalletError::IdentityBuildFailed);
    }
    if !removals.is_empty() {
        let actual_removals = actual_map
            .get(REMOVE_VDXF_KEY)
            .and_then(Value::as_array)
            .ok_or(WalletError::IdentityBuildFailed)?;
        let expected = removals
            .iter()
            .map(normalize_vdxf_univalue)
            .collect::<Result<Vec<_>, _>>()?;
        let actual = actual_removals
            .iter()
            .map(normalize_vdxf_univalue)
            .collect::<Result<Vec<_>, _>>()?;
        if expected != actual {
            return Err(WalletError::IdentityBuildFailed);
        }
    }
    let mut groups = Vec::new();
    for (changed, key, bytes, mime, digest) in [
        (
            avatar_changes,
            AVATAR_VDXF_KEY,
            avatar_bytes.as_deref(),
            AVATAR_MIME,
            &avatar_digest,
        ),
        (
            header_changes,
            HEADER_VDXF_KEY,
            header_bytes.as_deref(),
            AVATAR_MIME,
            &header_digest,
        ),
        (
            description_changes,
            DESCRIPTION_VDXF_KEY,
            normalized_description
                .as_ref()
                .filter(|value| !value.is_empty())
                .map(|value| value.as_bytes()),
            DESCRIPTION_MIME,
            &description_digest,
        ),
    ] {
        if !changed {
            continue;
        }
        let Some(bytes) = bytes else {
            continue;
        };
        let resolved = resolve_profile_payload(
            &generated_descriptors(actual_map, key)?,
            &decoded,
            &[mime],
            &system_id,
            &request.identity_address,
            &primary_addresses,
            Network::Testnet,
        )?;
        groups.push(ProfileEvidenceGroup {
            field: if key == AVATAR_VDXF_KEY {
                "avatar"
            } else if key == HEADER_VDXF_KEY {
                "header"
            } else {
                "description"
            }
            .into(),
            first_output: resolved.evidence_vout,
            parts: resolved.evidence_parts,
            encoded_bytes: bytes.len(),
        });
        if resolved.bytes != bytes || Some(hex::encode(resolved.digest)).as_ref() != digest.as_ref()
        {
            return Err(WalletError::IdentityBuildFailed);
        }
    }

    let mut template_tx = decode_verus_tx(&template_hex)?;
    // Bound ambiguous-broadcast recovery even if a daemon template has no expiry.
    template_tx.expiry_height = height
        .checked_add(20)
        .ok_or(WalletError::IdentityBuildFailed)?;
    validate_template_layout(&template_tx, &decoded, &groups)?;
    let (fee_sats, evidence_bytes) =
        calculate_fee(&template_tx, &decoded, &actual_control, *export_fee_sats)?;
    let profile_intent = ProfileTransactionIntent {
        identity_txid: target.txid.clone(),
        identity_vout: target.vout,
        output_scripts: template_tx
            .outputs
            .iter()
            .map(|output| hex::encode(&output.script_pub_key))
            .collect(),
        output_values: template_tx
            .outputs
            .iter()
            .map(|output| output.value)
            .collect(),
        avatar_digest,
        header_digest,
        description_digest,
        previous_profile: current.clone(),
        proposed_profile: proposed,
        publication: None,
    };
    Ok(PreparedTemplate {
        tx: template_tx,
        control: actual_control,
        intent: profile_intent,
        fee_sats,
        evidence_bytes,
        groups,
        changed_fields,
    })
}

// Reject unknown, overlapping or unaccounted-for evidence; classify authenticated
// descriptor groups, never encoded image lengths. Bind RPC decoding to raw scripts.
fn validate_template_layout(
    tx: &crate::core::channels::vrpc::identity::verus_tx::model::VerusTx,
    decoded: &Value,
    groups: &[ProfileEvidenceGroup],
) -> Result<(), WalletError> {
    let outputs = decoded
        .get("vout")
        .and_then(Value::as_array)
        .ok_or(WalletError::IdentityBuildFailed)?;
    if outputs.len() != tx.outputs.len() || tx.outputs.iter().any(|output| output.value != 0) {
        return Err(WalletError::IdentityBuildFailed);
    }
    let mut covered = std::collections::HashSet::new();
    let mut fields = std::collections::HashSet::new();
    for group in groups {
        if !fields.insert(&group.field)
            || !["avatar", "header", "description"].contains(&group.field.as_str())
            || (group.field == "description" && group.parts != 1)
        {
            return Err(WalletError::IdentityBuildFailed);
        }
        if group.parts == 0 || group.parts > super::codec::MAX_EVIDENCE_PARTS {
            return Err(WalletError::IdentityBuildFailed);
        }
        for index in group.first_output
            ..group
                .first_output
                .checked_add(group.parts)
                .ok_or(WalletError::IdentityBuildFailed)?
        {
            if !covered.insert(index)
                || outputs
                    .get(index)
                    .and_then(|out| out.pointer("/scriptPubKey/notaryevidence"))
                    .is_none()
            {
                return Err(WalletError::IdentityBuildFailed);
            }
        }
    }
    for (index, (output, raw)) in outputs.iter().zip(&tx.outputs).enumerate() {
        if output.pointer("/scriptPubKey/hex").and_then(Value::as_str)
            != Some(hex::encode(&raw.script_pub_key).as_str())
            || (output.pointer("/scriptPubKey/notaryevidence").is_some()
                != covered.contains(&index))
            || (!covered.contains(&index)
                && output.pointer("/scriptPubKey/identityprimary").is_none())
        {
            return Err(WalletError::IdentityBuildFailed);
        }
    }
    Ok(())
}

pub(crate) async fn finish_preflight(
    mut prepared: PreparedTemplate,
    request: &IdentityProfilePreflightRequest,
    publication: ProfilePublicationReview,
    preflight_store: &PreflightStore,
    account_id: &str,
    session_id: &str,
    from_address: &str,
    channel_id: &str,
    provider: &VrpcProvider,
) -> Result<IdentityProfilePreflightResult, WalletError> {
    if prepared.multipart_images() > 1 {
        return Err(WalletError::IdentityBuildFailed);
    }
    let current_target = parse_target_identity(
        provider
            .getidentity_for_profile(&request.identity_address)
            .await?,
    )?;
    if current_target.txid != prepared.intent.identity_txid
        || current_target.vout != prepared.intent.identity_vout
    {
        return Err(WalletError::InvalidPreflight);
    }
    let fee_sats = prepared.fee_sats;
    let funding_candidates = parse_funding_utxos(
        &provider
            .getaddressutxos(&[from_address.to_string()])
            .await?,
    );
    if total_satoshis(&funding_candidates) < fee_sats {
        return Err(WalletError::InsufficientFunds);
    }
    let (identity_script, identity_satoshis) = fetch_identity_prevout(
        provider,
        &prepared.intent.identity_txid,
        prepared.intent.identity_vout,
    )
    .await?;
    let (unsigned_hex, signable_inputs, _) = build_unsigned_identity_tx(
        &mut prepared.tx,
        &prepared.intent.identity_txid,
        prepared.intent.identity_vout,
        &identity_script,
        identity_satoshis,
        &funding_candidates,
        fee_sats,
    )?;
    let authenticated_inputs = signable_inputs
        .iter()
        .map(|input| VrpcInputRef {
            txid: input.txid.clone(),
            vout: input.vout,
            satoshis: input.satoshis,
            script_pub_key: Some(input.script_pub_key.clone()),
        })
        .collect::<Vec<_>>();
    authenticate_payload_inputs(provider, &authenticated_inputs).await?;
    let input_total = authenticated_inputs
        .iter()
        .try_fold(0i64, |total, input| total.checked_add(input.satoshis))
        .ok_or(WalletError::IdentityBuildFailed)?;
    validate_profile_intent(
        &unsigned_hex,
        &prepared.intent,
        from_address,
        input_total,
        fee_sats,
    )?;

    let fee = sat_to_decimal_string(fee_sats);
    let preflight_id = Uuid::new_v4().to_string();
    let payload = IdentityPreflightPayload {
        unsigned_hex,
        signable_inputs,
        operation: IdentityOperation::Update,
        target_identity: request.identity_address.clone(),
        from_address: from_address.to_string(),
        fee: fee.clone(),
        memo: None,
        control_intent: prepared.control,
        profile_intent: Some(prepared.intent.clone()),
    };
    if !preflight_store.put_with_ttl(
        preflight_id.clone(),
        PreflightRecord {
            session_id: session_id.to_string(),
            channel_id: channel_id.to_string(),
            account_id: account_id.to_string(),
            payload: serde_json::to_value(payload).map_err(|_| WalletError::IdentityBuildFailed)?,
        },
        Some(IDENTITY_PREFLIGHT_TTL),
    ) {
        return Err(WalletError::WalletLocked);
    }
    let expires_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| WalletError::OperationFailed)?
        .as_secs()
        + IDENTITY_PREFLIGHT_TTL.as_secs();
    Ok(IdentityProfilePreflightResult {
        preflight_id,
        expires_at,
        current_profile: prepared.intent.previous_profile,
        proposed_profile: prepared.intent.proposed_profile,
        fee_sats: fee_sats.to_string(),
        fee_display: fee,
        funding_summary: from_address.to_string(),
        evidence_bytes: prepared.evidence_bytes,
        changed_fields: prepared.changed_fields,
        publication,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::channels::vrpc::identity::verus_tx::model::{
        VerusTx, VerusTxOut, SAPLING_VERSION_GROUP_ID,
    };
    use crate::core::channels::vrpc::intent::IdentityControlIntent;

    fn empty_control() -> IdentityControlIntent {
        IdentityControlIntent {
            version: 3,
            flags: 0,
            primary_addresses: vec![vec![1; 20]],
            minimum_signatures: 1,
            parent: vec![2; 20],
            name: b"profile".to_vec(),
            content_multimap: Vec::new(),
            legacy_content_map: Vec::new(),
            content_map: Vec::new(),
            revocation_authority: vec![3; 20],
            recovery_authority: vec![4; 20],
            private_addresses: Vec::new(),
            system_id: Some(vec![5; 20]),
            unlock_after: None,
        }
    }

    #[test]
    fn layout_classifies_real_groups_and_rejects_overlap_or_unbound_outputs() {
        let make = |avatar_parts: usize, header_parts: usize| {
            let count = 1 + avatar_parts + header_parts + 1;
            let tx = VerusTx {
                version: 4,
                overwintered: true,
                version_group_id: SAPLING_VERSION_GROUP_ID,
                inputs: vec![],
                outputs: (0..count)
                    .map(|index| VerusTxOut {
                        value: 0,
                        script_pub_key: vec![index as u8],
                    })
                    .collect(),
                lock_time: 0,
                expiry_height: 0,
                value_balance: 0,
            };
            let decoded = json!({"vout": (0..count).map(|index| if index == 0 { json!({"scriptPubKey":{"hex":"00","identityprimary":{}}}) } else { json!({"scriptPubKey":{"hex":hex::encode([index as u8]),"notaryevidence":{}}}) }).collect::<Vec<_>>()});
            let groups = vec![
                ProfileEvidenceGroup {
                    field: "avatar".into(),
                    first_output: 1,
                    parts: avatar_parts,
                    encoded_bytes: 10_000,
                },
                ProfileEvidenceGroup {
                    field: "header".into(),
                    first_output: 1 + avatar_parts,
                    parts: header_parts,
                    encoded_bytes: 10_000,
                },
                ProfileEvidenceGroup {
                    field: "description".into(),
                    first_output: count - 1,
                    parts: 1,
                    encoded_bytes: 10,
                },
            ];
            (tx, decoded, groups)
        };
        for (avatar, header, multipart) in [(1, 1, 0), (2, 1, 1), (1, 6, 1), (2, 3, 2)] {
            let (tx, decoded, groups) = make(avatar, header);
            validate_template_layout(&tx, &decoded, &groups).unwrap();
            assert_eq!(
                groups.iter().filter(|group| group.parts > 1).count(),
                multipart
            );
        }
        let (tx, mut decoded, mut groups) = make(2, 2);
        groups[1].first_output = 2;
        assert!(validate_template_layout(&tx, &decoded, &groups).is_err());
        groups[1].first_output = 3;
        decoded["vout"][1]["scriptPubKey"]["hex"] = json!("ff");
        assert!(validate_template_layout(&tx, &decoded, &groups).is_err());
        let (mut tx, decoded, groups) = make(1, 1);
        tx.outputs[1].value = 1;
        assert!(validate_template_layout(&tx, &decoded, &groups).is_err());
    }

    #[test]
    fn image_changes_distinguish_keep_noop_replace_and_remove() {
        use IdentityProfileAvatarChange::*;
        let encoded = BASE64_STANDARD.encode(b"image");
        assert!(!image_changes(&Keep, None, Some(&encoded)));
        assert!(!image_changes(
            &Set {
                value: encoded.clone(),
                mime_type: AVATAR_MIME.into()
            },
            Some(b"image"),
            Some(&encoded)
        ));
        assert!(image_changes(
            &Set {
                value: encoded.clone(),
                mime_type: AVATAR_MIME.into()
            },
            Some(b"new image"),
            Some(&encoded)
        ));
        assert!(image_changes(&Remove, None, Some(&encoded)));
        assert!(!image_changes(&Remove, None, None));
    }

    #[test]
    fn header_set_is_validated_before_signing_and_keep_remove_need_no_image() {
        assert!(
            decode_image_change(&IdentityProfileAvatarChange::Keep, true)
                .unwrap()
                .is_none()
        );
        assert!(
            decode_image_change(&IdentityProfileAvatarChange::Remove, true)
                .unwrap()
                .is_none()
        );
        assert!(matches!(
            decode_image_change(
                &IdentityProfileAvatarChange::Set {
                    value: "?".into(),
                    mime_type: AVATAR_MIME.into()
                },
                true
            ),
            Err(WalletError::IdentityProfileInvalidHeader)
        ));
        let mut bytes = Vec::new();
        image::codecs::webp::WebPEncoder::new_lossless(&mut bytes)
            .encode(
                &vec![0; 960 * 160 * 3],
                960,
                160,
                image::ExtendedColorType::Rgb8,
            )
            .unwrap();
        let change = IdentityProfileAvatarChange::Set {
            value: BASE64_STANDARD.encode(&bytes),
            mime_type: AVATAR_MIME.into(),
        };
        assert_eq!(decode_image_change(&change, true).unwrap(), Some(bytes));
        assert!(decode_image_change(&change, false).is_err());
    }

    #[test]
    fn description_normalization_uses_graphemes_and_utf8_byte_bounds() {
        assert_eq!(
            normalize_description("  first\r\nsecond  ").unwrap(),
            "first\nsecond"
        );
        assert!(normalize_description(&"a".repeat(160)).is_ok());
        assert!(normalize_description(&"a".repeat(161)).is_err());
        assert!(normalize_description(&"é".repeat(160)).is_ok());
    }

    #[test]
    fn signed_data_wrapper_preserves_the_locally_signed_mmr_salt() {
        let (wrapped, digest) = signed_data_value(
            "iSduGc7La416e3SfLD17tCe4Qvreg2i6br",
            VRSCTEST_SYSTEM_ID,
            b"profile",
            DESCRIPTION_MIME,
            "description",
            1_234_567,
            &[1u8; 32],
        )
        .unwrap();
        let data = &wrapped["data"];

        assert_eq!(data["mmrdata"]["serializedhex"], "70726f66696c65");
        assert_eq!(data["mmrdata"]["label"], "description");
        assert_eq!(data["mmrdata"]["mimetype"], DESCRIPTION_MIME);
        assert!(data.get("messagehex").is_none());
        assert!(data.get("serializedbase64").is_none());
        assert_eq!(data["mmrsalt"].as_str().map(str::len), Some(64));
        assert!(!data["signature"].as_str().unwrap().is_empty());
        assert_eq!(digest.len(), 64);
    }

    #[test]
    fn transaction_export_fee_uses_exact_decimal_arithmetic() {
        assert_eq!(decimal_coin_to_sats("0.01").unwrap(), 1_000_000);
        assert_eq!(decimal_coin_to_sats("0.00000001").unwrap(), 1);
        assert_eq!(
            transaction_export_fee_sats(&json!({ "transactionexportfee": 0.01 })).unwrap(),
            1_000_000
        );
        assert_eq!(
            transaction_export_fee_sats(&json!({ "transactionexportfee": "1.25" })).unwrap(),
            125_000_000
        );
        assert!(decimal_coin_to_sats("0.000000001").is_err());
        assert!(decimal_coin_to_sats("-0.01").is_err());
        assert!(decimal_coin_to_sats("0").is_err());
    }

    #[test]
    fn storage_fee_formula_is_integer_and_counts_evidence_scripts() {
        let template = VerusTx {
            version: 4,
            overwintered: true,
            version_group_id: SAPLING_VERSION_GROUP_ID,
            inputs: Vec::new(),
            outputs: vec![
                VerusTxOut {
                    value: 0,
                    script_pub_key: vec![0; 100],
                },
                VerusTxOut {
                    value: 0,
                    script_pub_key: vec![0; 6_000],
                },
                VerusTxOut {
                    value: 0,
                    script_pub_key: vec![0; 500],
                },
            ],
            lock_time: 0,
            expiry_height: 0,
            value_balance: 0,
        };
        let decoded = json!({
            "vout": [
                {"scriptPubKey": {}},
                {"scriptPubKey": {"notaryevidence": {}}},
                {"scriptPubKey": {}}
            ]
        });

        let (fee, evidence_bytes) =
            calculate_fee(&template, &decoded, &empty_control(), 1_000_000).unwrap();

        assert_eq!(evidence_bytes, 6_500);
        assert_eq!(fee, 6_550_000);
    }

    #[test]
    fn transaction_export_fee_requires_a_positive_supported_value() {
        assert_eq!(
            transaction_export_fee_sats(&json!({"transactionexportfee": "0.01"})).unwrap(),
            1_000_000
        );
        assert!(transaction_export_fee_sats(&json!({"transactionexportfee": 0})).is_err());
        assert!(transaction_export_fee_sats(&json!({})).is_err());
    }

    #[test]
    fn removal_targets_one_exact_value_with_action_two() {
        let removed = removal_value(AVATAR_VDXF_KEY, &json!({"serializedhex": "0102"})).unwrap();
        let payload = &removed[REMOVE_VDXF_KEY];
        assert_eq!(payload["version"], 1);
        assert_eq!(payload["action"], 2);
        assert_eq!(payload["entrykey"], AVATAR_VDXF_KEY);
        assert_eq!(payload["valuehash"].as_str().map(str::len), Some(64));
    }
}
