//
// Main Tauri application setup and command registration
// Security: Manages application state and exposes secure wallet command interface
// Last Updated: Network-aware provider pools for mainnet/testnet routing

mod commands;
mod core;
mod types;

use commands::{
    address_book, bridge_transfer, clipboard, coins, generic_request, guard, identity, transaction,
    vrpc_transfer, wallet,
};
use core::auth::kdf::derive_current_argon2id;
use core::channels::btc::BtcProviderPool;
use core::channels::eth::EthProviderPool;
use core::channels::vrpc::VrpcProviderPool;
use core::{
    AccountStateStore, CoinRegistry, GuardSessionManager, PreflightStore, SessionManager,
    StrongholdStore, UpdateEngine, WalletManager,
};
use std::path::Path;
use std::sync::Arc;
use tauri::Manager;
#[cfg(any(target_os = "linux", all(debug_assertions, windows)))]
use tauri_plugin_deep_link::DeepLinkExt;
use tokio::sync::Mutex;

#[cfg(debug_assertions)]
const DEBUG_STRONGHOLD_WORK_FACTOR_ENV: &str = "VERUS_STRONGHOLD_ENCRYPT_WORK_FACTOR";
#[cfg(debug_assertions)]
const DEBUG_STRONGHOLD_ENCRYPT_WORK_FACTOR: u8 = 15;

#[cfg(debug_assertions)]
fn load_runtime_env_files() {
    // Load local env files for desktop dev/runtime convenience without committing secrets.
    // Existing process environment variables keep precedence.
    for candidate in [
        ".env.local",
        ".env",
        ".env.zcash-params",
        "../.env.local",
        "../.env",
        "../.env.zcash-params",
    ] {
        if !Path::new(candidate).exists() {
            continue;
        }

        match dotenvy::from_filename(candidate) {
            Ok(_) => println!("[APP] Loaded runtime env file: {}", candidate),
            Err(err) => eprintln!(
                "[APP] Failed to load runtime env file {}: {}",
                candidate, err
            ),
        }
    }
}

#[cfg(not(debug_assertions))]
fn load_runtime_env_files() {}

#[cfg(debug_assertions)]
fn configure_stronghold_encrypt_work_factor() {
    let configured = std::env::var(DEBUG_STRONGHOLD_WORK_FACTOR_ENV)
        .ok()
        .and_then(|value| value.trim().parse::<u8>().ok())
        .unwrap_or(DEBUG_STRONGHOLD_ENCRYPT_WORK_FACTOR);

    match iota_stronghold::engine::snapshot::try_set_encrypt_work_factor(configured) {
        Ok(()) => {
            println!(
                "[APP] Stronghold encrypt work factor set to {} (debug build)",
                configured
            );
            if configured < 15 {
                eprintln!(
                    "[APP] Warning: Stronghold work factor {} is below secure floor (15)",
                    configured
                );
            }
        }
        Err(error) => {
            eprintln!(
                "[APP] Invalid Stronghold work factor {} from {}: {:?}",
                configured, DEBUG_STRONGHOLD_WORK_FACTOR_ENV, error
            );
        }
    }
}

#[cfg(not(debug_assertions))]
fn configure_stronghold_encrypt_work_factor() {}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    load_runtime_env_files();
    configure_stronghold_encrypt_work_factor();

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            #[cfg(any(target_os = "linux", all(debug_assertions, windows)))]
            app.deep_link().register_all()?;

            // Initialize wallet manager with app data dir (unified with Stronghold storage)
            let app_dir = app.path().app_data_dir().map_err(|e| {
                eprintln!("[APP] Failed to get app data directory: {:?}", e);
                e
            })?;
            let wallet_data_dir = app_dir.join("wallet_data");
            std::fs::create_dir_all(&wallet_data_dir).map_err(|e| {
                eprintln!("[APP] Failed to create wallet data directory: {:?}", e);
                e
            })?;
            let wallet_manager = WalletManager::new(wallet_data_dir.clone());
            let any_current_kdf_accounts = wallet_manager
                .has_account_with_key_derivation_version(
                    StrongholdStore::current_key_derivation_version_static(),
                )
                .map_err(|e| {
                    eprintln!(
                        "[APP] Failed to inspect wallet key derivation versions: {:?}",
                        e
                    );
                    e
                })?;
            let app_local_dir = app.path().app_local_data_dir().map_err(|e| {
                eprintln!("[APP] Failed to get app local data directory: {:?}", e);
                e
            })?;
            let argon2_salt_path = core::auth::kdf::argon2_salt_path(&app_local_dir);
            let allow_create_salt = !any_current_kdf_accounts;
            app.handle().plugin(
                tauri_plugin_stronghold::Builder::new(move |password| {
                    derive_current_argon2id(password, &argon2_salt_path, allow_create_salt)
                        .expect("failed to derive Stronghold Argon2id hash")
                })
                .build(),
            )?;
            app.manage(wallet_manager);

            let account_state_store = AccountStateStore::new(wallet_data_dir.clone());
            app.manage(account_state_store);

            // Initialize Stronghold store and session manager
            let app_handle = app.handle();
            let stronghold_store = StrongholdStore::new(app_handle).map_err(|e| {
                eprintln!("[APP] Failed to initialize Stronghold store: {:?}", e);
                e
            })?;
            let session_manager = Arc::new(Mutex::new(SessionManager::new(stronghold_store)));
            app.manage(session_manager);
            println!("[APP] Session manager initialized");

            let coin_registry_store = wallet_data_dir.join("dynamic_coins_by_account.json");
            let coin_registry = Arc::new(CoinRegistry::with_dynamic_store(coin_registry_store));
            app.manage(coin_registry);
            println!("[APP] Coin registry initialized");

            let preflight_store = PreflightStore::new();
            app.manage(preflight_store);
            println!("[APP] Preflight store initialized");

            let guard_session_manager = Arc::new(Mutex::new(GuardSessionManager::new()));
            app.manage(guard_session_manager);
            println!("[APP] Guard session manager initialized");

            let vrpc_provider_pool = Arc::new(VrpcProviderPool::new());
            app.manage(vrpc_provider_pool);
            println!("[APP] VRPC providers initialized");

            let btc_provider_pool = Arc::new(BtcProviderPool::new());
            app.manage(btc_provider_pool);
            println!("[APP] BTC providers initialized");

            let eth_provider_pool = Arc::new(EthProviderPool::new());
            if let Some(reason) = eth_provider_pool.disabled_reason() {
                println!("[APP] ETH providers disabled: {}", reason);
            } else {
                println!("[APP] ETH providers initialized");
            }
            app.manage(eth_provider_pool);

            let update_engine = Arc::new(UpdateEngine::new());
            app.manage(update_engine);
            println!("[APP] Update engine initialized");

            println!("[APP] Wallet manager initialized");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Wallet creation commands
            wallet::generate_mnemonic,
            wallet::validate_mnemonic,
            wallet::get_mnemonic_wordlist,
            wallet::create_wallet,
            wallet::import_wallet_text,
            wallet::list_wallets,
            wallet::get_active_wallet,
            // Session management commands
            wallet::unlock_wallet,
            wallet::start_update_engine,
            wallet::lock_wallet,
            wallet::get_addresses,
            wallet::get_coin_scopes,
            wallet::get_active_assets,
            wallet::set_active_assets,
            wallet::get_dlight_seed_status,
            wallet::setup_dlight_seed,
            wallet::get_wallet_recovery_secrets,
            wallet::get_dlight_runtime_status,
            wallet::get_dlight_prover_status,
            wallet::get_session_timeout_minutes,
            wallet::set_session_timeout_minutes,
            wallet::get_watched_vrpc_addresses,
            wallet::set_watched_vrpc_addresses,
            wallet::is_unlocked,
            clipboard::read_clipboard_text,
            // Coin registry commands (Module 3)
            coins::get_coin_registry,
            coins::add_coin_definition,
            coins::add_pbaas_currency,
            coins::resolve_pbaas_currency,
            coins::resolve_erc20_contract,
            // Transaction commands (Module 4 + 9)
            transaction::preflight_send,
            transaction::send_transaction,
            transaction::get_balances,
            transaction::get_transaction_history,
            transaction::get_transaction_history_page,
            vrpc_transfer::preflight_vrpc_transfer,
            bridge_transfer::get_bridge_capabilities,
            bridge_transfer::get_bridge_conversion_paths,
            bridge_transfer::estimate_bridge_conversion,
            bridge_transfer::estimate_bridge_export_fee,
            bridge_transfer::preflight_bridge_transfer,
            // Identity commands
            identity::preflight_identity_update,
            identity::send_identity_update,
            identity::discover_linkable_identities,
            identity::get_linked_identities,
            identity::link_identity,
            identity::unlink_identity,
            identity::set_linked_identity_favorite,
            identity::get_identity_details,
            generic_request::verify_generic_request_signature,
            generic_request::sign_generic_response,
            generic_request::build_and_sign_generic_response,
            generic_request::post_generic_response_callback,
            generic_request::open_generic_request_callback,
            generic_request::sign_identity_signature_hash,
            generic_request::verify_identity_signature_hash,
            generic_request::review_generic_identity_update,
            generic_request::preflight_generic_identity_update,
            generic_request::store_generic_provisioning_job,
            generic_request::list_identity_provisioning_jobs,
            generic_request::refresh_identity_provisioning_jobs,
            generic_request::link_ready_identity_provisioning,
            guard::begin_guard_session,
            guard::end_guard_session,
            guard::lookup_guard_target_identity,
            guard::preflight_guard_identity_update,
            guard::send_guard_identity_update,
            // Address book commands
            address_book::list_address_book_contacts,
            address_book::save_address_book_contact,
            address_book::delete_address_book_contact,
            address_book::mark_address_book_endpoint_used,
            address_book::validate_destination_address,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
