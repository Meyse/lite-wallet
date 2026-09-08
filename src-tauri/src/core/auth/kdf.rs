use std::path::{Path, PathBuf};

use argon2::{Algorithm, Argon2, Params, Version};
use rand::rngs::OsRng;
use rand::RngCore;

use crate::types::errors::WalletError;

pub const LEGACY_KEY_DERIVATION_VERSION: u8 = 1;
pub const CURRENT_KEY_DERIVATION_VERSION: u8 = 2;
pub const ARGON2_SALT_LEN: usize = 32;
pub const ARGON2_HASH_LEN: usize = 32;

const ARGON2_MEMORY_KIB: u32 = 19_456;
const ARGON2_TIME_COST: u32 = 2;
const ARGON2_LANES: u32 = 1;

pub fn derive_legacy_sha256(password: &str) -> Vec<u8> {
    use sha2::{Digest, Sha256};

    let mut hasher = Sha256::new();
    hasher.update(password.as_bytes());
    hasher.finalize().to_vec()
}

pub fn argon2_salt_path(app_local_data_dir: &Path) -> PathBuf {
    app_local_data_dir
        .join("stronghold")
        .join("argon2_salt.bin")
}

pub fn derive_current_argon2id(
    password: &str,
    salt_path: &Path,
    allow_create: bool,
) -> Result<Vec<u8>, WalletError> {
    let salt = load_or_create_salt(salt_path, allow_create)?;
    derive_argon2id_with_salt(password, &salt)
}

pub fn derive_argon2id_with_salt(password: &str, salt: &[u8]) -> Result<Vec<u8>, WalletError> {
    let params = Params::new(
        ARGON2_MEMORY_KIB,
        ARGON2_TIME_COST,
        ARGON2_LANES,
        Some(ARGON2_HASH_LEN),
    )
    .map_err(|_| WalletError::OperationFailed)?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut hash = vec![0_u8; ARGON2_HASH_LEN];
    argon2
        .hash_password_into(password.as_bytes(), salt, &mut hash)
        .map_err(|_| WalletError::OperationFailed)?;
    Ok(hash)
}

fn load_or_create_salt(
    salt_path: &Path,
    allow_create: bool,
) -> Result<[u8; ARGON2_SALT_LEN], WalletError> {
    if salt_path.is_file() {
        let bytes = std::fs::read(salt_path).map_err(|_| WalletError::OperationFailed)?;
        if bytes.len() != ARGON2_SALT_LEN {
            return Err(WalletError::SecureStorageUnavailable);
        }
        let mut salt = [0_u8; ARGON2_SALT_LEN];
        salt.copy_from_slice(&bytes);
        return Ok(salt);
    }

    if !allow_create {
        return Err(WalletError::SecureStorageUnavailable);
    }

    let parent = salt_path.parent().ok_or(WalletError::OperationFailed)?;
    std::fs::create_dir_all(parent).map_err(|_| WalletError::OperationFailed)?;

    let mut salt = [0_u8; ARGON2_SALT_LEN];
    OsRng.fill_bytes(&mut salt);
    std::fs::write(salt_path, salt).map_err(|_| WalletError::OperationFailed)?;
    Ok(salt)
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::{
        derive_argon2id_with_salt, derive_current_argon2id, derive_legacy_sha256, ARGON2_SALT_LEN,
    };
    use crate::types::errors::WalletError;

    #[test]
    fn legacy_sha256_is_stable() {
        assert_eq!(
            derive_legacy_sha256("secret"),
            derive_legacy_sha256("secret")
        );
        assert_ne!(
            derive_legacy_sha256("secret"),
            derive_legacy_sha256("secret2")
        );
    }

    #[test]
    fn argon2id_with_fixed_salt_is_deterministic() {
        let salt = [7_u8; ARGON2_SALT_LEN];
        let first = derive_argon2id_with_salt("secret", &salt).expect("argon2 hash");
        let second = derive_argon2id_with_salt("secret", &salt).expect("argon2 hash");
        let different = derive_argon2id_with_salt("secret2", &salt).expect("argon2 hash");
        assert_eq!(first, second);
        assert_ne!(first, different);
    }

    #[test]
    fn missing_salt_without_create_returns_secure_storage_unavailable() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let salt_path = std::env::temp_dir()
            .join(format!("lite_wallet_argon2_missing_salt_{unique}"))
            .join("argon2_salt.bin");

        let result = derive_current_argon2id("secret", &salt_path, false);
        assert!(matches!(result, Err(WalletError::SecureStorageUnavailable)));
    }
}
