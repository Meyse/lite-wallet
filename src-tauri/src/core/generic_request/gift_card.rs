//! Gift cards: a request's spendable key detail (`SpendableKeyDetails`), as
//! Verus Mobile writes and reads it at `dev-asf-parent` (`seedDetails.js`,
//! `crypto.js`, `spendableKey.js`).
//!
//! The detail carries 32 bytes of BIP39 entropy, plain or encrypted with a
//! password. The 24 words become the claim key the way Verus Mobile turns a
//! seed into a key (`derive_keys_v1`). Claiming moves the key's VerusIDs and
//! funds into this wallet.

use aes_gcm::aead::Aead;
use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
use zeroize::Zeroizing;

use crate::types::WalletError;

use super::acceptance::Rejection;
use super::wire::{invalid, Reader};

const FLAG_ENCRYPTED: u64 = 1;
const FLAG_CONTAINS_KDF_ITERS: u64 = 2;
const SEED_FORMAT_BIP39: u64 = 1;
const ENCRYPTION_FORMAT_NONE: u64 = 0;
const ENCRYPTION_FORMAT_SALTED_TAGGED_AES_256_GCM: u64 = 1;
/// `SEED_DETAILS_ENCRYPTION_ITERS_MAX`.
const MAX_KDF_ITERATIONS: u64 = 1_000_000;
const ENTROPY_LENGTH: usize = 32;
const SALT_LENGTH: usize = 16;
const IV_LENGTH: usize = 12;
const TAG_LENGTH: usize = 16;

/// `SeedDetails`, as `SpendableKeyDetails` serializes it.
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct SpendableKey {
    flags: u64,
    seed_format: u64,
    encryption_format: u64,
    kdf_iterations: u64,
    data: Vec<u8>,
}

impl std::fmt::Debug for SpendableKey {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SpendableKey")
            .field("encrypted", &self.requires_password())
            .finish_non_exhaustive()
    }
}

pub(crate) fn parse_spendable_key(data: &[u8]) -> Result<SpendableKey, WalletError> {
    let mut reader = Reader::new(data);
    let flags = reader.compact_size()?;
    let seed_format = reader.compact_size()?;
    let encryption_format = reader.compact_size()?;
    let kdf_iterations = if flags & FLAG_CONTAINS_KDF_ITERS != 0 {
        reader.compact_size()?
    } else {
        0
    };
    let secret = reader.var_slice()?.to_vec();
    reader.finish()?;
    Ok(SpendableKey {
        flags,
        seed_format,
        encryption_format,
        kdf_iterations,
        data: secret,
    })
}

impl SpendableKey {
    /// `seedDetailsRequiresPassword`.
    pub(crate) fn requires_password(&self) -> bool {
        self.flags & FLAG_ENCRYPTED != 0
            || self.encryption_format == ENCRYPTION_FORMAT_SALTED_TAGGED_AES_256_GCM
    }

    /// `validateSeedDetails`: BIP39 only; encrypted details use salted,
    /// tagged AES-256-GCM with a bounded iteration count.
    pub(crate) fn check(&self) -> Result<(), Rejection> {
        if self.seed_format != SEED_FORMAT_BIP39 {
            return Err(Rejection::GiftCardInvalid);
        }
        if self.requires_password() {
            if self.encryption_format != ENCRYPTION_FORMAT_SALTED_TAGGED_AES_256_GCM
                || self.flags & FLAG_CONTAINS_KDF_ITERS == 0
                || self.kdf_iterations == 0
                || self.kdf_iterations > MAX_KDF_ITERATIONS
            {
                return Err(Rejection::GiftCardInvalid);
            }
        } else if self.encryption_format != ENCRYPTION_FORMAT_NONE {
            return Err(Rejection::GiftCardInvalid);
        }
        Ok(())
    }

    /// `seedDetailsOrdinalToMnemonic`: the 24 words, decrypted with the
    /// password when the card has one.
    pub(crate) fn mnemonic(
        &self,
        password: Option<&str>,
    ) -> Result<Zeroizing<String>, WalletError> {
        self.check()?;
        let entropy = if self.requires_password() {
            let password = password
                .filter(|password| !password.is_empty())
                .ok_or(Rejection::GiftCardPassword)?;
            decrypt(password, &self.data, self.kdf_iterations)?
        } else {
            Zeroizing::new(self.data.clone())
        };
        if entropy.len() != ENTROPY_LENGTH {
            return Err(Rejection::GiftCardInvalid.into());
        }
        let mnemonic = bip39::Mnemonic::from_entropy_in(bip39::Language::English, &entropy)
            .map_err(|_| WalletError::from(Rejection::GiftCardInvalid))?;
        Ok(Zeroizing::new(mnemonic.to_string()))
    }
}

/// `saltedDecryptToBuffer`: salt, IV, ciphertext, and tag; the key is
/// PBKDF2-HMAC-SHA256 of the UTF-8 password. A wrong password fails the tag.
fn decrypt(
    password: &str,
    blob: &[u8],
    iterations: u64,
) -> Result<Zeroizing<Vec<u8>>, WalletError> {
    if blob.len() < SALT_LENGTH + IV_LENGTH + TAG_LENGTH {
        return Err(Rejection::GiftCardInvalid.into());
    }
    let (salt, rest) = blob.split_at(SALT_LENGTH);
    let (iv, sealed) = rest.split_at(IV_LENGTH);
    let rounds = u32::try_from(iterations).map_err(|_| invalid())?;
    let nonce: [u8; IV_LENGTH] = iv.try_into().map_err(|_| invalid())?;
    let mut key = Zeroizing::new([0u8; 32]);
    pbkdf2::pbkdf2_hmac::<sha2::Sha256>(password.as_bytes(), salt, rounds, key.as_mut_slice());
    Aes256Gcm::new_from_slice(key.as_slice())
        .map_err(|_| invalid())?
        .decrypt(&Nonce::from(nonce), sealed)
        .map(Zeroizing::new)
        .map_err(|_| Rejection::GiftCardPassword.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::generic_request::request::parse_request_hex;
    use serde_json::Value;

    fn vectors() -> Value {
        let raw = include_str!("../../../../src/lib/genericRequest/fixtures/request-vectors.json");
        serde_json::from_str::<Value>(raw).unwrap()["giftCards"].clone()
    }

    fn card(name: &str) -> SpendableKey {
        let request = parse_request_hex(vectors()[name].as_str().unwrap()).unwrap();
        assert_eq!(request.details.len(), 1);
        parse_spendable_key(&request.details[0].data).unwrap()
    }

    #[test]
    fn a_plain_gift_card_gives_its_words() {
        let card = card("plain");
        assert!(!card.requires_password());
        assert_eq!(
            card.mnemonic(None).unwrap().as_str(),
            vectors()["mnemonic"].as_str().unwrap()
        );
    }

    #[test]
    fn an_encrypted_gift_card_needs_its_password() {
        let vectors = vectors();
        let card = card("encrypted");
        assert!(card.requires_password());
        assert_eq!(
            card.mnemonic(vectors["password"].as_str())
                .unwrap()
                .as_str(),
            vectors["mnemonic"].as_str().unwrap()
        );
        for wrong in [None, Some(""), Some("wrong password")] {
            assert!(matches!(
                card.mnemonic(wrong),
                Err(WalletError::GenericRequestRejected(code)) if code == "giftCardPassword"
            ));
        }
    }

    #[test]
    fn unsupported_gift_card_formats_are_refused() {
        let mut card = card("encrypted");
        card.kdf_iterations = MAX_KDF_ITERATIONS + 1;
        assert_eq!(card.check(), Err(Rejection::GiftCardInvalid));
        let mut plain = super::tests::card("plain");
        plain.encryption_format = ENCRYPTION_FORMAT_SALTED_TAGGED_AES_256_GCM;
        plain.flags = 0;
        assert_eq!(plain.check(), Err(Rejection::GiftCardInvalid));
    }
}
