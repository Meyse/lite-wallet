//! Gift card claims (`SpendableKeyDetails`), as Verus Mobile claims them at
//! `dev-asf-parent` (`SpendableKeyRequestInfo.js`, `spendableKey.js`).
//!
//! Rust reads the key from the verified request, scans what it holds on the
//! wallet's chain, and moves its VerusIDs and funds to the wallet's primary
//! address. The key and the signed transactions stay in memory with the open
//! request; a retry broadcasts the same transactions, never new ones. A claim
//! sends no response.

use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::State;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::commands::generic_request::{private_account_key, wallet_network_to_crypto_network};
use crate::commands::identity::{parse_discovery_candidates, parse_getidentity_payload};
use crate::core::auth::{capture_active_wallet_access_context, SessionManager};
use crate::core::channels::vrpc::claim::{
    claim_totals, identity_fee_outputs, parse_claim_outputs, preflight_identity_claim,
    preflight_sweep, sign_claim, split_fee_outputs, ClaimOutput,
};
use crate::core::channels::vrpc::common::VrpcPreflightPayload;
use crate::core::channels::vrpc::identity::preflight::{IdentityPreflightPayload, DEFAULT_FEE_SAT};
use crate::core::channels::vrpc::{sat_to_decimal_string, VrpcProvider, VrpcProviderPool};
use crate::core::channels::PreflightRecord;
use crate::core::crypto::{
    derive_private_scalar_from_material, derive_public_profile_from_private_bytes,
};
use crate::core::generic_request::acceptance::Rejection;
use crate::core::generic_request::details::DetailKind;
use crate::core::generic_request::gift_card::parse_spendable_key;
use crate::core::generic_request::names;
use crate::core::generic_request::store::{GenericRequestStore, GiftCardClaim, OpenRequest};
use crate::core::PreflightStore;
use crate::types::wallet::{WalletNetwork, WalletSecretKind};
use crate::types::{
    GiftCardClaimPreflight, GiftCardClaimResult, GiftCardCurrency, GiftCardIdentity,
    GiftCardReview, WalletError,
};

/// A claim must be approved within the time any preflight lives.
const CLAIM_PREFLIGHT_TTL: Duration = Duration::from_secs(15 * 60);
const CLAIM_CHANNEL: &str = "vrpc.giftcard";

#[derive(Serialize, Deserialize)]
struct ClaimPayload {
    identities: Vec<IdentityPreflightPayload>,
    sweep: Option<VrpcPreflightPayload>,
}

fn open_spendable_key(
    open: &OpenRequest,
) -> Result<crate::core::generic_request::gift_card::SpendableKey, WalletError> {
    let detail = open
        .parsed
        .details
        .iter()
        .zip(&open.kinds)
        .find(|(_, kind)| **kind == DetailKind::SpendableKey)
        .map(|(detail, _)| detail)
        .ok_or(WalletError::GenericRequestInvalidEnvelope)?;
    parse_spendable_key(&detail.data)
}

/// A VerusID the claim key controls, as the scan reads it.
struct ScannedIdentity {
    review: GiftCardIdentity,
}

async fn scan_identities(
    provider: &VrpcProvider,
    claim_address: &str,
) -> Result<Vec<ScannedIdentity>, WalletError> {
    let found = parse_discovery_candidates(
        provider
            .getidentitieswithaddress(claim_address, true)
            .await?,
    );
    let mut identities = Vec::new();
    for candidate in found {
        let parsed =
            parse_getidentity_payload(provider.getidentity(&candidate.identity_address).await?)?;
        let identity = &parsed.identity;
        let text = |field: &str| {
            identity
                .get(field)
                .and_then(Value::as_str)
                .unwrap_or_default()
        };
        let primaries = identity
            .get("primaryaddresses")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        // Only VerusIDs the key controls (`spendableKey.js`).
        if !primaries
            .iter()
            .any(|value| value.as_str() == Some(claim_address))
        {
            continue;
        }
        let address = text("identityaddress").to_string();
        let claimable = parsed.status.as_deref() == Some("active")
            && identity.get("minimumsignatures").and_then(Value::as_u64) == Some(1);
        identities.push(ScannedIdentity {
            review: GiftCardIdentity {
                external_authorities: text("revocationauthority") != address
                    || text("recoveryauthority") != address,
                identity_address: address,
                fully_qualified_name: parsed.fully_qualified_name.clone(),
                claimable,
            },
        });
    }
    Ok(identities)
}

async fn scan_outputs(
    provider: &VrpcProvider,
    claim_address: &str,
    system_id: &str,
) -> Result<Vec<ClaimOutput>, WalletError> {
    Ok(parse_claim_outputs(
        &provider.funding_utxos(&[claim_address.to_string()]).await?,
        system_id,
    ))
}

async fn currency_name(provider: &VrpcProvider, currency_id: &str) -> Option<String> {
    let currency = provider.getcurrency(currency_id).await.ok()?;
    ["fullyqualifiedname", "name"]
        .iter()
        .find_map(|field| currency.get(*field).and_then(Value::as_str))
        .map(str::to_string)
}

/// Reads the gift card's key and what it holds. With a password card, the
/// password decrypts the key; it is never stored.
#[tauri::command(rename_all = "snake_case")]
pub async fn review_gift_card(
    request_hex: String,
    password: Option<String>,
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
    vrpc_provider_pool: State<'_, Arc<VrpcProviderPool>>,
    request_store: State<'_, GenericRequestStore>,
) -> Result<GiftCardReview, WalletError> {
    let context = capture_active_wallet_access_context(session_manager.inner()).await?;
    let open = request_store.get(&context.session_id, &request_hex)?;
    let card = open_spendable_key(&open)?;
    let mnemonic = card.mnemonic(password.as_deref())?;
    let private_key = derive_private_scalar_from_material(&mnemonic, WalletSecretKind::SeedText)?;
    let network = wallet_network_to_crypto_network(context.wallet_network);
    let claim_address = derive_public_profile_from_private_bytes(&private_key, network)?.address;

    let system_id = names::root_system_id(matches!(context.wallet_network, WalletNetwork::Testnet));
    let provider = vrpc_provider_pool.for_system(context.wallet_network, system_id);
    let outputs = scan_outputs(provider, &claim_address, system_id).await?;
    let identities = scan_identities(provider, &claim_address).await?;

    let (native, others) = claim_totals(&outputs);
    let mut currencies = Vec::new();
    if native > 0 {
        currencies.push(GiftCardCurrency {
            currency_id: system_id.to_string(),
            name: currency_name(provider, system_id).await,
            amount: sat_to_decimal_string(native),
        });
    }
    for (currency_id, sats) in others {
        currencies.push(GiftCardCurrency {
            name: currency_name(provider, &currency_id).await,
            currency_id,
            amount: sat_to_decimal_string(sats),
        });
    }

    request_store.update_claim(&context.session_id, &open.parsed.request_hash, |claim| {
        *claim = Some(GiftCardClaim {
            address: claim_address.clone(),
            private_key: zeroize::Zeroizing::new(*private_key),
            signed: Vec::new(),
            broadcast: Vec::new(),
        });
    });
    Ok(GiftCardReview {
        claim_address,
        currencies,
        identities: identities
            .into_iter()
            .map(|identity| identity.review)
            .collect(),
    })
}

/// Builds and checks the claim: an update for each VerusID the key controls,
/// then a sweep of every other output to the wallet's primary address.
#[tauri::command(rename_all = "snake_case")]
pub async fn preflight_gift_card_claim(
    request_hex: String,
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
    preflight_store: State<'_, PreflightStore>,
    vrpc_provider_pool: State<'_, Arc<VrpcProviderPool>>,
    request_store: State<'_, GenericRequestStore>,
) -> Result<GiftCardClaimPreflight, WalletError> {
    let context = capture_active_wallet_access_context(session_manager.inner()).await?;
    let open = request_store.get(&context.session_id, &request_hex)?;
    let claim = open.claim.clone().ok_or(WalletError::InvalidPreflight)?;
    let system_id = names::root_system_id(matches!(context.wallet_network, WalletNetwork::Testnet));
    let provider = vrpc_provider_pool.for_system(context.wallet_network, system_id);

    let identities = scan_identities(provider, &claim.address).await?;
    if identities.iter().any(|identity| !identity.review.claimable) {
        return Err(Rejection::GiftCardIdentity.into());
    }
    let outputs = scan_outputs(provider, &claim.address, system_id).await?;
    let (fee_outputs, rest) = split_fee_outputs(&outputs, identities.len());

    // The wallet's private address replaces the card's; without one, the
    // claimed VerusID keeps none (Verus Mobile's default).
    let wallet_private_address = match private_account_key(&context).await {
        Ok(account) => {
            let (_, address) = account
                .to_diversifiable_full_viewing_key()
                .default_address();
            names::sapling_address(&address.to_bytes())
        }
        Err(WalletError::GenericRequestRejected(_)) => None,
        Err(error) => return Err(error),
    };

    let mut identity_payloads = Vec::new();
    for (index, identity) in identities.iter().enumerate() {
        let identity_address = &identity.review.identity_address;
        // Without a plain native output of the key, the VerusID pays its own
        // fee, as Verus Mobile does for a card that only gives a VerusID.
        let fee_candidates = match fee_outputs.get(index) {
            Some(output) => vec![output.clone()],
            None => identity_fee_outputs(
                &provider
                    .funding_utxos(std::slice::from_ref(identity_address))
                    .await?,
                system_id,
                identity_address,
            ),
        };
        if fee_candidates.is_empty() {
            return Err(WalletError::InsufficientFunds);
        }
        identity_payloads.push(
            preflight_identity_claim(
                provider,
                identity_address,
                &claim.address,
                &context.vrsc_address,
                wallet_private_address.as_deref(),
                &fee_candidates,
            )
            .await?,
        );
    }
    let sweep = preflight_sweep(
        provider,
        &claim.address,
        &context.vrsc_address,
        &rest,
        system_id,
    )
    .await?;
    if identity_payloads.is_empty() && sweep.is_none() {
        return Err(Rejection::GiftCardEmpty.into());
    }

    let fee_sat = DEFAULT_FEE_SAT * identity_payloads.len() as i64
        + sweep
            .as_ref()
            .map(|sweep| crate::core::channels::vrpc::parse_positive_amount_sat(&sweep.fee))
            .transpose()?
            .unwrap_or(0);
    let transaction_count = identity_payloads.len() + usize::from(sweep.is_some());
    let preflight_id = Uuid::new_v4().to_string();
    let payload = serde_json::to_value(ClaimPayload {
        identities: identity_payloads,
        sweep,
    })
    .map_err(|_| WalletError::OperationFailed)?;
    if !preflight_store.put_with_ttl(
        preflight_id.clone(),
        PreflightRecord {
            session_id: context.session_id,
            channel_id: CLAIM_CHANNEL.to_string(),
            account_id: context.account_id,
            payload,
        },
        Some(CLAIM_PREFLIGHT_TTL),
    ) {
        return Err(WalletError::WalletLocked);
    }
    Ok(GiftCardClaimPreflight {
        preflight_id,
        fee: sat_to_decimal_string(fee_sat),
        fee_currency_id: system_id.to_string(),
        transaction_count,
    })
}

/// The ID of a signed transaction: double SHA-256 of its bytes, reversed.
fn txid_of(signed_hex: &str) -> Result<String, WalletError> {
    let bytes = hex::decode(signed_hex).map_err(|_| WalletError::OperationFailed)?;
    let mut hash = names::sha256d(&[&bytes]);
    hash.reverse();
    Ok(hex::encode(hash))
}

/// Signs the claim with the card's key and broadcasts it in order. After a
/// failure, the next call sends the transactions that were not sent yet.
#[tauri::command(rename_all = "snake_case")]
pub async fn send_gift_card_claim(
    request_hex: String,
    preflight_id: String,
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
    preflight_store: State<'_, PreflightStore>,
    vrpc_provider_pool: State<'_, Arc<VrpcProviderPool>>,
    request_store: State<'_, GenericRequestStore>,
) -> Result<GiftCardClaimResult, WalletError> {
    let context = capture_active_wallet_access_context(session_manager.inner()).await?;
    let open = request_store.get(&context.session_id, &request_hex)?;
    let mut claim = open.claim.clone().ok_or(WalletError::InvalidPreflight)?;
    let system_id = names::root_system_id(matches!(context.wallet_network, WalletNetwork::Testnet));
    let provider = vrpc_provider_pool.for_system(context.wallet_network, system_id);

    if claim.signed.is_empty() {
        let record = preflight_store
            .take(&preflight_id, &context.session_id)
            .ok_or(WalletError::InvalidPreflight)?;
        if record.channel_id != CLAIM_CHANNEL || record.account_id != context.account_id {
            return Err(WalletError::InvalidPreflight);
        }
        let payload: ClaimPayload =
            serde_json::from_value(record.payload).map_err(|_| WalletError::InvalidPreflight)?;
        claim.signed = sign_claim(
            &context.vrsc_address,
            &payload.identities,
            payload.sweep.as_ref(),
            &claim.private_key,
        )?;
        let signed = claim.signed.clone();
        request_store.update_claim(&context.session_id, &open.parsed.request_hash, |stored| {
            if let Some(stored) = stored {
                stored.signed = signed;
            }
        });
    }

    for signed_hex in claim.signed.iter().skip(claim.broadcast.len()) {
        let expected = txid_of(signed_hex)?;
        let sent = match provider.sendrawtransaction(signed_hex).await {
            Ok(result) => crate::core::channels::vrpc::common::parse_txid_from_result(&result)
                .ok_or(WalletError::OperationFailed)?,
            // A send that reached the node before an error is already there.
            Err(error) => match provider.getrawtransaction(&expected, 0).await {
                Ok(_) => expected.clone(),
                Err(_) => return Err(error),
            },
        };
        if !sent.eq_ignore_ascii_case(&expected) {
            return Err(WalletError::OperationFailed);
        }
        claim.broadcast.push(expected.clone());
        request_store.update_claim(&context.session_id, &open.parsed.request_hash, |stored| {
            if let Some(stored) = stored {
                stored.broadcast.push(expected);
            }
        });
    }
    if let Some(last) = claim.broadcast.last() {
        request_store.record_broadcast(&context.session_id, &open.parsed.request_hash, last);
    }
    Ok(GiftCardClaimResult {
        txids: claim.broadcast,
    })
}

#[cfg(test)]
mod tests {
    use super::txid_of;

    #[test]
    fn a_txid_is_the_reversed_double_sha256_of_the_transaction() {
        // Bitcoin's genesis coinbase transaction.
        let genesis = "01000000010000000000000000000000000000000000000000000000000000000000000000ffffffff4d04ffff001d0104455468652054696d65732030332f4a616e2f32303039204368616e63656c6c6f72206f6e206272696e6b206f66207365636f6e64206261696c6f757420666f722062616e6b73ffffffff0100f2052a01000000434104678afdb0fe5548271967f1a67130b7105cd6a828e03909a67962e0ea1f61deb649f6bc3f4cef38c4f35504e51ec112de5c384df7ba0b8d578a4c702b6bf11d5fac00000000";
        assert_eq!(
            txid_of(genesis).unwrap(),
            "4a5e1e4baab89f3a32518a88c31bc87f618f76673e2cc77ab2127b7afdeda33b"
        );
    }
}
