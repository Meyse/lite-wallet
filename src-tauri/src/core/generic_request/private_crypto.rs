//! Encryption keys and encrypted payloads for requests, ported from VerusCoin's
//! `verus_zfunc` (`z_getencryptionaddress`, `encrypt_data`, `decrypt_data`),
//! the native code behind Verus Mobile's encryption. The port runs on the
//! wallet's own Sapling crates; `fixtures/verus-zfunc-vectors.json` holds
//! vectors from the reference for a fixed public test seed.

use chacha20poly1305::aead::Aead;
use chacha20poly1305::{ChaCha20Poly1305, KeyInit, Nonce};
use rand::RngCore;
use sapling::note_encryption::{PreparedIncomingViewingKey, SaplingDomain};
use sapling::value::NoteValue;
use sapling::zip32::ExtendedSpendingKey;
use sapling::{Note, PaymentAddress, Rseed, SaplingIvk};
use sha2::{Digest, Sha256};
use zcash_note_encryption::{Domain, EphemeralKeyBytes};
use zeroize::Zeroizing;
use zip32::{ChildIndex, Scope};

use crate::types::WalletError;

const VERUS_COIN_TYPE: u32 = 133;
const EXTENDED_KEY_LENGTH: usize = 169;

/// The keys of one encryption channel between two identities.
pub(crate) struct ChannelKeys {
    pub address: PaymentAddress,
    /// The extended full viewing key, serialized (169 bytes).
    pub extended_viewing_key: [u8; EXTENDED_KEY_LENGTH],
    /// The incoming viewing key scalar, little-endian as the curve encodes it.
    /// Verus shows it byte-reversed ("display order").
    pub incoming_viewing_key: [u8; 32],
    /// The channel's extended spending key, when asked for.
    pub extended_spending_key: Option<Zeroizing<[u8; EXTENDED_KEY_LENGTH]>>,
}

/// `z_getencryptionaddress` from an account spending key: a channel seed is
/// SHA-256 of the account key, the from-ID (or one zero byte without one),
/// and the to-ID when present; the channel key is m/32'/133'/index' under it.
pub(crate) fn channel_keys(
    account: &ExtendedSpendingKey,
    from_id: Option<&[u8; 20]>,
    to_id: Option<&[u8; 20]>,
    encryption_index: u32,
    return_secret: bool,
) -> Result<ChannelKeys, WalletError> {
    // `ChildIndex::hardened` panics from 2^31.
    if encryption_index >= 1 << 31 {
        return Err(WalletError::OperationFailed);
    }
    let base = Zeroizing::new(account.to_bytes());
    let mut hasher = Sha256::new();
    hasher.update(base.as_slice());
    match from_id {
        Some(id) => hasher.update(id),
        None => hasher.update([0u8]),
    }
    if let Some(id) = to_id {
        hasher.update(id);
    }
    let channel_seed = Zeroizing::new(<[u8; 32]>::from(hasher.finalize()));

    let channel = ExtendedSpendingKey::from_path(
        &ExtendedSpendingKey::master(channel_seed.as_slice()),
        &[
            ChildIndex::hardened(32),
            ChildIndex::hardened(VERUS_COIN_TYPE),
            ChildIndex::hardened(encryption_index),
        ],
    );

    let mut extended_viewing_key = [0u8; EXTENDED_KEY_LENGTH];
    #[allow(deprecated)]
    channel
        .to_extended_full_viewing_key()
        .write(extended_viewing_key.as_mut_slice())
        .map_err(|_| WalletError::OperationFailed)?;
    let viewing_key = channel.to_diversifiable_full_viewing_key();
    let (_, address) = viewing_key.default_address();

    Ok(ChannelKeys {
        address,
        extended_viewing_key,
        incoming_viewing_key: viewing_key.to_ivk(Scope::External).0.to_bytes(),
        extended_spending_key: return_secret.then(|| Zeroizing::new(channel.to_bytes())),
    })
}

/// An encrypted payload: the sender's ephemeral public key and the
/// ChaCha20-Poly1305 ciphertext.
pub(crate) struct EncryptedPayload {
    pub ephemeral_public_key: [u8; 32],
    pub ciphertext: Vec<u8>,
}

fn cipher(symmetric_key: &[u8]) -> Result<ChaCha20Poly1305, WalletError> {
    ChaCha20Poly1305::new_from_slice(symmetric_key).map_err(|_| WalletError::OperationFailed)
}

/// `encrypt_data`: a one-time Sapling key agreement with `address`, then
/// ChaCha20-Poly1305 with the all-zero nonce (the key is never reused).
pub(crate) fn encrypt_to_address(
    address: &PaymentAddress,
    plaintext: &[u8],
) -> Result<EncryptedPayload, WalletError> {
    let mut rseed = Zeroizing::new([0u8; 32]);
    rand::rngs::OsRng.fill_bytes(rseed.as_mut_slice());
    let note = Note::from_parts(*address, NoteValue::from_raw(0), Rseed::AfterZip212(*rseed));
    // With a ZIP 212 rseed the ephemeral key is derived from it; the RNG is
    // not used.
    let esk = note.generate_or_derive_esk(&mut rand::rngs::OsRng);
    let epk = <SaplingDomain as Domain>::ka_derive_public(&note, &esk);
    let epk_bytes = <SaplingDomain as Domain>::epk_bytes(&epk);
    let shared = <SaplingDomain as Domain>::ka_agree_enc(&esk, address.pk_d());
    let key = <SaplingDomain as Domain>::kdf(shared, &epk_bytes);
    let ciphertext = cipher(&key.as_bytes()[..32])?
        .encrypt(&Nonce::default(), plaintext)
        .map_err(|_| WalletError::OperationFailed)?;
    Ok(EncryptedPayload {
        ephemeral_public_key: epk_bytes.0,
        ciphertext,
    })
}

/// `decrypt_data` with an incoming viewing key (little-endian scalar) and the
/// sender's ephemeral public key.
pub(crate) fn decrypt_with_viewing_key(
    incoming_viewing_key: &[u8; 32],
    ephemeral_public_key: &[u8; 32],
    ciphertext: &[u8],
) -> Result<Zeroizing<Vec<u8>>, WalletError> {
    let scalar = Option::<jubjub::Fr>::from(jubjub::Fr::from_bytes(incoming_viewing_key))
        .ok_or(WalletError::OperationFailed)?;
    let ivk = PreparedIncomingViewingKey::new(&SaplingIvk(scalar));
    let epk_bytes = EphemeralKeyBytes(*ephemeral_public_key);
    let epk = <SaplingDomain as Domain>::epk(&epk_bytes).ok_or(WalletError::OperationFailed)?;
    let shared =
        <SaplingDomain as Domain>::ka_agree_dec(&ivk, &<SaplingDomain as Domain>::prepare_epk(epk));
    let key = <SaplingDomain as Domain>::kdf(shared, &epk_bytes);
    decrypt_with_symmetric_key(&key.as_bytes()[..32], ciphertext)
}

/// `decrypt_data` with the symmetric key a payload may carry.
pub(crate) fn decrypt_with_symmetric_key(
    symmetric_key: &[u8],
    ciphertext: &[u8],
) -> Result<Zeroizing<Vec<u8>>, WalletError> {
    cipher(symmetric_key)?
        .decrypt(&Nonce::default(), ciphertext)
        .map(Zeroizing::new)
        .map_err(|_| WalletError::OperationFailed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn vectors() -> Value {
        serde_json::from_str(include_str!("fixtures/verus-zfunc-vectors.json")).unwrap()
    }

    fn account(vectors: &Value) -> ExtendedSpendingKey {
        let bytes = hex::decode(vectors["accountExtsk"].as_str().unwrap()).unwrap();
        ExtendedSpendingKey::from_bytes(&bytes)
            .ok()
            .expect("account key parses")
    }

    fn id(value: &Value) -> Option<[u8; 20]> {
        value
            .as_str()
            .map(|hex_value| hex::decode(hex_value).unwrap().try_into().unwrap())
    }

    #[test]
    fn a_channel_number_from_2_pow_31_is_an_error() {
        let vectors = vectors();
        let account = account(&vectors);
        assert!(channel_keys(&account, None, None, 0x7fff_ffff, false).is_ok());
        assert!(channel_keys(&account, None, None, 0x8000_0000, false).is_err());
    }

    #[test]
    fn channel_keys_match_verus_zfunc() {
        let vectors = vectors();
        let account = account(&vectors);
        for case in vectors["channels"].as_array().unwrap() {
            let from = id(&case["fromId"]);
            let to = id(&case["toId"]);
            let keys = channel_keys(
                &account,
                from.as_ref(),
                to.as_ref(),
                case["index"].as_u64().unwrap() as u32,
                true,
            )
            .unwrap();
            assert_eq!(
                crate::core::generic_request::names::sapling_address(&keys.address.to_bytes())
                    .unwrap(),
                case["address"].as_str().unwrap()
            );
            assert_eq!(
                hex::encode(keys.incoming_viewing_key),
                case["ivkRaw"].as_str().unwrap()
            );
            assert_eq!(
                hex::encode(keys.extended_viewing_key),
                case["extfvk"].as_str().unwrap()
            );
            assert_eq!(
                hex::encode(keys.extended_spending_key.unwrap().as_slice()),
                case["extsk"].as_str().unwrap()
            );
        }
    }

    #[test]
    fn a_reference_payload_decrypts_with_the_channel_viewing_key() {
        let vectors = vectors();
        let encrypted = &vectors["encrypted"];
        let case = &vectors["channels"][encrypted["toCase"].as_u64().unwrap() as usize];
        let ivk: [u8; 32] = hex::decode(case["ivkRaw"].as_str().unwrap())
            .unwrap()
            .try_into()
            .unwrap();
        let epk: [u8; 32] = hex::decode(encrypted["epk"].as_str().unwrap())
            .unwrap()
            .try_into()
            .unwrap();
        let ciphertext = hex::decode(encrypted["ciphertext"].as_str().unwrap()).unwrap();
        let plaintext = decrypt_with_viewing_key(&ivk, &epk, &ciphertext).unwrap();
        assert_eq!(
            hex::encode(plaintext.as_slice()),
            encrypted["plaintextHex"].as_str().unwrap()
        );
        let symmetric = hex::decode(encrypted["symmetricKey"].as_str().unwrap()).unwrap();
        assert_eq!(
            decrypt_with_symmetric_key(&symmetric, &ciphertext)
                .unwrap()
                .as_slice(),
            plaintext.as_slice()
        );
    }

    #[test]
    fn a_payload_encrypted_here_decrypts_with_the_recipient_key_only() {
        let vectors = vectors();
        let account = account(&vectors);
        let recipient = channel_keys(&account, Some(&[1; 20]), Some(&[2; 20]), 3, false).unwrap();
        let other = channel_keys(&account, Some(&[1; 20]), Some(&[2; 20]), 4, false).unwrap();
        let payload = encrypt_to_address(&recipient.address, b"for the recipient").unwrap();
        assert_eq!(
            decrypt_with_viewing_key(
                &recipient.incoming_viewing_key,
                &payload.ephemeral_public_key,
                &payload.ciphertext
            )
            .unwrap()
            .as_slice(),
            b"for the recipient"
        );
        assert!(decrypt_with_viewing_key(
            &other.incoming_viewing_key,
            &payload.ephemeral_public_key,
            &payload.ciphertext
        )
        .is_err());
    }
}
