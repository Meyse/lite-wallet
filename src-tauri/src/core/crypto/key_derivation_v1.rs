//
// Verus-Mobile v1 key derivation implementation
// Security: Implements exact parity with Verus-Mobile newsend3 branch (sha256+iguana clamp)
// Last Updated: Added Bitcoin P2PKH address (same key, version byte 0x00) for Verus-Mobile parity

use crate::core::crypto::eth_keys::derive_eth_address;
use crate::core::crypto::wif_encoding::{
    decode_wif_unchecked_network, encode_wif, generate_p2pkh_address,
    generate_p2pkh_address_with_version, Network,
};
use crate::types::errors::WalletError;
use crate::types::wallet::{DerivedKeys, WalletSecretKind};
use secp256k1::{PublicKey, Secp256k1, SecretKey};
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

/// Bitcoin mainnet P2PKH version byte (addresses start with "1")
const BITCOIN_P2PKH_VERSION_MAINNET: u8 = 0x00;
/// Bitcoin testnet P2PKH version byte (addresses start with "m" or "n")
const BITCOIN_P2PKH_VERSION_TESTNET: u8 = 0x6F;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DerivedPublicProfile {
    pub address: String,
    pub pub_hex: String,
    pub eth_address: String,
    pub btc_address: String,
}

/// Derive keys using Verus-Mobile v1 derivation method
///
/// Flow:
/// 1. SHA256(seed string as UTF-8) → 32 bytes
/// 2. Apply iguana clamp bit operations
/// 3. Validate scalar is in secp256k1 curve range [1, n-1]
/// 4. Derive compressed secp256k1 public key
/// 5. Encode WIF and P2PKH address
/// 6. Derive ETH keys from same scalar
///
/// Security: Never logs seed or derived keys
pub fn derive_keys_v1(seed: &str, network: Network) -> Result<DerivedKeys, WalletError> {
    let private_scalar = derive_private_scalar_from_seed(seed)?;
    derive_keys_from_private_bytes(&private_scalar, network)
}

/// Derive keys from existing secret material format.
pub fn derive_keys_from_material(
    secret: &str,
    secret_kind: WalletSecretKind,
    network: Network,
) -> Result<DerivedKeys, WalletError> {
    let private_scalar = derive_private_scalar_from_material(secret, secret_kind)?;
    derive_keys_from_private_bytes(&private_scalar, network)
}

pub fn derive_private_scalar_from_material(
    secret: &str,
    secret_kind: WalletSecretKind,
) -> Result<Zeroizing<[u8; 32]>, WalletError> {
    let private_scalar = match secret_kind {
        WalletSecretKind::SeedText => derive_private_scalar_from_seed(secret)?,
        WalletSecretKind::Wif => Zeroizing::new(decode_wif_unchecked_network(secret)?),
        WalletSecretKind::PrivateKeyHex => Zeroizing::new(decode_private_key_hex(secret)?),
    };
    validate_private_scalar(&private_scalar)?;
    Ok(private_scalar)
}

pub fn derive_public_profile_from_material(
    secret: &str,
    secret_kind: WalletSecretKind,
    network: Network,
) -> Result<DerivedPublicProfile, WalletError> {
    let private_scalar = derive_private_scalar_from_material(secret, secret_kind)?;
    derive_public_profile_from_private_bytes(&private_scalar, network)
}

pub fn derive_public_profile_from_private_bytes(
    priv_bytes: &[u8; 32],
    network: Network,
) -> Result<DerivedPublicProfile, WalletError> {
    let scalar = validate_private_scalar(priv_bytes)?;
    let secp = Secp256k1::new();
    let pub_key = PublicKey::from_secret_key(&secp, &scalar);
    let pub_hex = hex::encode(pub_key.serialize());
    let address = generate_p2pkh_address(&pub_key, network)?;
    let eth_address = derive_eth_address(&pub_key)?;
    let btc_version = match network {
        Network::Mainnet => BITCOIN_P2PKH_VERSION_MAINNET,
        Network::Testnet => BITCOIN_P2PKH_VERSION_TESTNET,
    };
    let btc_address = generate_p2pkh_address_with_version(&pub_key, btc_version)?;

    Ok(DerivedPublicProfile {
        address,
        pub_hex,
        eth_address,
        btc_address,
    })
}

pub fn derive_private_scalar_from_seed(seed: &str) -> Result<Zeroizing<[u8; 32]>, WalletError> {
    // 1. SHA256(seed string as UTF-8 bytes) - no normalization
    let mut priv_bytes = sha256_digest(seed.as_bytes());

    // 2. Apply iguana clamp (ensures scalar is in valid range)
    // bytes[0] &= 248  (clear bottom 3 bits)
    priv_bytes[0] &= 248;
    // bytes[31] &= 127 (clear top bit)
    priv_bytes[31] &= 127;
    // bytes[31] |= 64  (set second-highest bit)
    priv_bytes[31] |= 64;

    validate_private_scalar(&priv_bytes)?;
    Ok(Zeroizing::new(priv_bytes))
}

/// Derive all wallet keys and addresses from a 32-byte private scalar.
pub fn derive_keys_from_private_bytes(
    priv_bytes: &[u8; 32],
    network: Network,
) -> Result<DerivedKeys, WalletError> {
    // 3. Validate scalar is in range [1, n-1] where n is curve order
    let scalar = validate_private_scalar(priv_bytes)?;

    // 4. Derive compressed secp256k1 public key (33 bytes)
    let secp = Secp256k1::new();
    let pub_key = PublicKey::from_secret_key(&secp, &scalar);
    let pub_hex = hex::encode(pub_key.serialize());

    // 5. Encode WIF
    let wif = encode_wif(priv_bytes, network)?;

    // 6. Generate P2PKH address
    let address = generate_p2pkh_address(&pub_key, network)?;

    // 7. Derive ETH keys from same scalar
    let eth_private_key = hex::encode(priv_bytes);
    let eth_address = derive_eth_address(&pub_key)?;

    // 8. Bitcoin P2PKH address (same key, network-matched version byte)
    let btc_version = match network {
        Network::Mainnet => BITCOIN_P2PKH_VERSION_MAINNET,
        Network::Testnet => BITCOIN_P2PKH_VERSION_TESTNET,
    };
    let btc_address = generate_p2pkh_address_with_version(&pub_key, btc_version)?;

    Ok(DerivedKeys {
        wif,
        address,
        pub_hex,
        eth_private_key,
        eth_address,
        btc_address,
    })
}

fn validate_private_scalar(priv_bytes: &[u8; 32]) -> Result<SecretKey, WalletError> {
    let scalar = SecretKey::from_slice(priv_bytes).map_err(|error| {
        // Never log the actual scalar value.
        WalletError::Internal(format!("Invalid scalar: {}", error))
    })?;

    if scalar.as_ref().iter().all(|&byte| byte == 0) {
        return Err(WalletError::Internal("Derived scalar is zero".to_string()));
    }

    Ok(scalar)
}

fn decode_private_key_hex(secret: &str) -> Result<[u8; 32], WalletError> {
    let stripped = secret
        .strip_prefix("0x")
        .or_else(|| secret.strip_prefix("0X"))
        .unwrap_or(secret);

    if stripped.len() != 64 {
        return Err(WalletError::InvalidAddress);
    }

    let decoded = hex::decode(stripped).map_err(|_| WalletError::InvalidAddress)?;
    if decoded.len() != 32 {
        return Err(WalletError::InvalidAddress);
    }

    let mut priv_bytes = [0u8; 32];
    priv_bytes.copy_from_slice(&decoded);
    Ok(priv_bytes)
}

/// Compute SHA256 digest of input bytes
fn sha256_digest(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();

    let mut bytes = [0u8; 32];
    bytes.copy_from_slice(&result);
    bytes
}
