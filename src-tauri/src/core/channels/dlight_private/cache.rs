//! Private note data is sensitive even though it cannot authorize a spend.
//! Keep it authenticated and encrypted, and commit a complete generation at once.
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use chacha20poly1305::{
    aead::{Aead, KeyInit, Payload},
    XChaCha20Poly1305, XNonce,
};
use fs2::FileExt;
use rand::{rngs::OsRng, RngCore};
use serde::{de::DeserializeOwned, Serialize};
use zeroize::Zeroizing;

use crate::types::WalletError;

const MAGIC: &[u8; 8] = b"LWZC0001";
const NONCE_LEN: usize = 24;
const MAX_CACHE_BYTES: u64 = 512 * 1024 * 1024;

pub(super) struct CacheStore {
    root: PathBuf,
    key: Zeroizing<[u8; 32]>,
    // Held until all operations for this unlock have stopped. A second process
    // must never migrate or write this cache while the first still owns it.
    _lock: File,
}

impl CacheStore {
    pub fn open(root: PathBuf, key: Zeroizing<[u8; 32]>) -> Result<Self, WalletError> {
        fs::create_dir_all(&root).map_err(|_| WalletError::OperationFailed)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&root, fs::Permissions::from_mode(0o700))
                .map_err(|_| WalletError::OperationFailed)?;
        }
        let lock = private_file(&root.join("private-wallet.lock"), false)?;
        lock.try_lock_exclusive()
            .map_err(|_| WalletError::DlightSpendCacheNotReady)?;
        Ok(Self {
            root,
            key,
            _lock: lock,
        })
    }

    pub fn load<T: DeserializeOwned>(&self) -> Result<Option<T>, WalletError> {
        let path = self.root.join("private-wallet.cache");
        if !path.exists() {
            return Ok(None);
        }
        let bytes = self.read_private(&path, b"state")?;
        serde_json::from_slice(&bytes)
            .map(Some)
            .map_err(|_| WalletError::OperationFailed)
    }

    pub fn save<T: Serialize>(&self, state: &T) -> Result<(), WalletError> {
        let bytes =
            Zeroizing::new(serde_json::to_vec(state).map_err(|_| WalletError::OperationFailed)?);
        self.write_private(&self.root.join("private-wallet.cache"), b"state", &bytes)
    }

    /// Preserve the exact legacy generations in an encrypted archive before
    /// removing their plaintext copies. An interrupted migration can be resumed.
    pub fn retire_legacy_files(&self, legacy_root: &Path) -> Result<(), WalletError> {
        const NAMES: &[&str] = &[
            "data.db",
            "blockmeta.db",
            "spend_wallet.db",
            "spend_meta.json",
        ];
        for name in NAMES {
            let source = legacy_root.join(name);
            if !source.exists() {
                continue;
            }
            let bytes = Zeroizing::new(read_bounded(&source)?);
            let archive = self.root.join(format!("{name}.legacy.enc"));
            if archive.exists() {
                if self.read_private(&archive, name.as_bytes())?.as_slice() != bytes.as_slice() {
                    return Err(WalletError::OperationFailed);
                }
            } else {
                self.write_private(&archive, name.as_bytes(), &bytes)?;
            }
            // Verify before retiring the original, including after an earlier crash.
            if self.read_private(&archive, name.as_bytes())?.as_slice() != bytes.as_slice() {
                return Err(WalletError::OperationFailed);
            }
            fs::remove_file(source).map_err(|_| WalletError::OperationFailed)?;
        }
        sync_directory(legacy_root)
    }

    fn write_private(&self, path: &Path, purpose: &[u8], bytes: &[u8]) -> Result<(), WalletError> {
        if bytes.len() as u64 > MAX_CACHE_BYTES - (MAGIC.len() + NONCE_LEN + 16) as u64 {
            return Err(WalletError::OperationFailed);
        }
        let cipher = XChaCha20Poly1305::new_from_slice(self.key.as_ref())
            .map_err(|_| WalletError::OperationFailed)?;
        let mut nonce = [0u8; NONCE_LEN];
        OsRng.fill_bytes(&mut nonce);
        let mut aad = MAGIC.to_vec();
        aad.extend_from_slice(purpose);
        let ciphertext = cipher
            .encrypt(
                XNonce::from_slice(&nonce),
                Payload {
                    msg: bytes,
                    aad: &aad,
                },
            )
            .map_err(|_| WalletError::OperationFailed)?;
        let mut envelope = Vec::with_capacity(MAGIC.len() + NONCE_LEN + ciphertext.len());
        envelope.extend_from_slice(MAGIC);
        envelope.extend_from_slice(&nonce);
        envelope.extend_from_slice(&ciphertext);
        atomic_write(path, &envelope)
    }

    fn read_private(&self, path: &Path, purpose: &[u8]) -> Result<Zeroizing<Vec<u8>>, WalletError> {
        let bytes = read_bounded(path)?;
        if bytes.len() < MAGIC.len() + NONCE_LEN + 16 || &bytes[..MAGIC.len()] != MAGIC {
            return Err(WalletError::OperationFailed);
        }
        let cipher = XChaCha20Poly1305::new_from_slice(self.key.as_ref())
            .map_err(|_| WalletError::OperationFailed)?;
        let mut aad = MAGIC.to_vec();
        aad.extend_from_slice(purpose);
        cipher
            .decrypt(
                XNonce::from_slice(&bytes[MAGIC.len()..MAGIC.len() + NONCE_LEN]),
                Payload {
                    msg: &bytes[MAGIC.len() + NONCE_LEN..],
                    aad: &aad,
                },
            )
            .map(Zeroizing::new)
            .map_err(|_| WalletError::OperationFailed)
    }
}

fn read_bounded(path: &Path) -> Result<Vec<u8>, WalletError> {
    if fs::metadata(path)
        .map_err(|_| WalletError::OperationFailed)?
        .len()
        > MAX_CACHE_BYTES
    {
        return Err(WalletError::OperationFailed);
    }
    fs::read(path).map_err(|_| WalletError::OperationFailed)
}

fn private_file(path: &Path, create_new: bool) -> Result<File, WalletError> {
    let mut options = OpenOptions::new();
    options.read(true).write(true);
    if create_new {
        options.create_new(true);
    } else {
        options.create(true).truncate(false);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(path).map_err(|_| WalletError::OperationFailed)
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), WalletError> {
    let root = path.parent().ok_or(WalletError::OperationFailed)?;
    let temporary = root.join(format!(".private-wallet-{}.tmp", uuid::Uuid::new_v4()));
    let result = (|| {
        let mut file = private_file(&temporary, true)?;
        file.write_all(bytes)
            .map_err(|_| WalletError::OperationFailed)?;
        file.sync_all().map_err(|_| WalletError::OperationFailed)?;
        drop(file);
        fs::rename(&temporary, path).map_err(|_| WalletError::OperationFailed)?;
        sync_directory(root)
    })();
    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    result
}

fn sync_directory(path: &Path) -> Result<(), WalletError> {
    #[cfg(unix)]
    {
        File::open(path)
            .and_then(|file| file.sync_all())
            .map_err(|_| WalletError::OperationFailed)?;
    }
    let _ = path;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root() -> PathBuf {
        std::env::temp_dir().join(format!("private-cache-test-{}", uuid::Uuid::new_v4()))
    }

    #[test]
    fn encrypted_roundtrip_wrong_key_and_tamper_fail_closed() {
        let root = root();
        let store = CacheStore::open(root.clone(), Zeroizing::new([7; 32])).unwrap();
        let value = serde_json::json!({"recipient":"private-recipient-marker", "amount":123});
        store.save(&value).unwrap();
        assert_eq!(store.load::<serde_json::Value>().unwrap(), Some(value));
        let path = root.join("private-wallet.cache");
        let mut bytes = fs::read(&path).unwrap();
        assert!(!String::from_utf8_lossy(&bytes).contains("private-recipient-marker"));
        assert!(CacheStore::open(root.clone(), Zeroizing::new([7; 32])).is_err());
        drop(store);
        let wrong = CacheStore::open(root.clone(), Zeroizing::new([8; 32])).unwrap();
        assert!(wrong.load::<serde_json::Value>().is_err());
        drop(wrong);
        let store = CacheStore::open(root.clone(), Zeroizing::new([7; 32])).unwrap();
        *bytes.last_mut().unwrap() ^= 1;
        fs::write(path, bytes).unwrap();
        assert!(store.load::<serde_json::Value>().is_err());
        drop(store);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn migration_archives_before_removing_plaintext_and_is_repeatable() {
        let root = root();
        let store = CacheStore::open(root.clone(), Zeroizing::new([7; 32])).unwrap();
        fs::write(root.join("data.db"), b"private legacy notes").unwrap();
        store.save(&serde_json::json!({"schemaVersion":1})).unwrap();
        store.retire_legacy_files(&root).unwrap();
        store.retire_legacy_files(&root).unwrap();
        assert!(!root.join("data.db").exists());
        assert_eq!(
            store
                .read_private(&root.join("data.db.legacy.enc"), b"data.db")
                .unwrap()
                .as_slice(),
            b"private legacy notes"
        );
        assert!(store
            .read_private(&root.join("data.db.legacy.enc"), b"state")
            .is_err());
        drop(store);
        fs::remove_dir_all(root).unwrap();
    }
}
