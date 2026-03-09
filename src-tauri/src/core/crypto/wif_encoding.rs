//
// WIF and P2PKH address encoding with Verus and Bitcoin network parameters
// Security: Implements Verus-Mobile compatible encoding (no sensitive data in logs)
// Last Updated: Added generate_p2pkh_address_with_version for Bitcoin P2PKH (version 0x00)

use crate::types::errors::WalletError;
use bs58;
use ripemd::Ripemd160;
use secp256k1::PublicKey;
use sha2::{Digest, Sha256};

const BITCOIN_WIF_VERSION_MAINNET: u8 = 0x80;
const BITCOIN_WIF_VERSION_TESTNET: u8 = 0xEF;

/// Network type for Verus (mainnet or testnet)
#[derive(Debug, Clone, Copy)]
pub enum Network {
    Mainnet,
    Testnet,
}

impl Network {
    /// WIF version byte for Verus network
    pub fn wif_version(&self) -> u8 {
        match self {
            Network::Mainnet => 0xBC, // 188
            // Verus testnet uses the same WIF prefix as Verus mainnet.
            Network::Testnet => 0xBC, // 188
        }
    }

    /// P2PKH address version byte for Verus network
    pub fn p2pkh_version(&self) -> u8 {
        match self {
            Network::Mainnet => 0x3C, // 60
            // Verus testnet uses R-addresses (same prefix byte as mainnet).
            Network::Testnet => 0x3C, // 60
        }
    }
}

/// Encode a 32-byte private key as WIF (Wallet Import Format)
/// Format: [version byte][32-byte key][0x01 compression flag] + checksum → base58check
pub fn encode_wif(priv_key: &[u8; 32], network: Network) -> Result<String, WalletError> {
    encode_wif_with_version(priv_key, network.wif_version())
}

/// Encode a 32-byte private key as WIF with an explicit version byte.
pub fn encode_wif_with_version(
    priv_key: &[u8; 32],
    version_byte: u8,
) -> Result<String, WalletError> {
    let mut data = Vec::with_capacity(34);
    data.push(version_byte);
    data.extend_from_slice(priv_key);
    data.push(0x01); // compressed flag

    Ok(bs58::encode(data).with_check().into_string())
}

/// Encode a Bitcoin network-matched WIF from a secp256k1 private key.
pub fn encode_btc_wif(priv_key: &[u8; 32], network: Network) -> Result<String, WalletError> {
    let version = match network {
        Network::Mainnet => BITCOIN_WIF_VERSION_MAINNET,
        Network::Testnet => BITCOIN_WIF_VERSION_TESTNET,
    };
    encode_wif_with_version(priv_key, version)
}

/// Decode a WIF string to extract the 32-byte private key
pub fn decode_wif(wif: &str, network: Network) -> Result<[u8; 32], WalletError> {
    let decoded = bs58::decode(wif)
        .with_check(None)
        .into_vec()
        .map_err(|_| WalletError::InvalidAddress)?;

    // Verify version byte
    if decoded.is_empty() || decoded[0] != network.wif_version() {
        return Err(WalletError::InvalidAddress);
    }

    // Extract 32-byte private key (skip version byte and compression flag)
    if decoded.len() < 34 {
        return Err(WalletError::InvalidAddress);
    }

    let mut priv_key = [0u8; 32];
    priv_key.copy_from_slice(&decoded[1..33]);

    Ok(priv_key)
}

/// Decode a WIF string without enforcing network version.
/// Accepts both compressed and uncompressed payload formats.
pub fn decode_wif_unchecked_network(wif: &str) -> Result<[u8; 32], WalletError> {
    let decoded = bs58::decode(wif)
        .with_check(None)
        .into_vec()
        .map_err(|_| WalletError::InvalidAddress)?;

    // [version][32-byte key] (uncompressed) OR [version][32-byte key][0x01] (compressed)
    if decoded.len() != 33 && decoded.len() != 34 {
        return Err(WalletError::InvalidAddress);
    }
    if decoded.len() == 34 && decoded[33] != 0x01 {
        return Err(WalletError::InvalidAddress);
    }

    let mut priv_key = [0u8; 32];
    priv_key.copy_from_slice(&decoded[1..33]);
    Ok(priv_key)
}

/// Generate a P2PKH address from a compressed public key
/// Format: [version byte][RIPEMD160(SHA256(pubkey))] + checksum → base58check
pub fn generate_p2pkh_address(
    pub_key: &PublicKey,
    network: Network,
) -> Result<String, WalletError> {
    generate_p2pkh_address_with_version(pub_key, network.p2pkh_version())
}

/// Generate a P2PKH address with an explicit version byte (e.g. Bitcoin mainnet 0x00)
/// Used for Bitcoin and any network that shares the same hash160 encoding.
pub fn generate_p2pkh_address_with_version(
    pub_key: &PublicKey,
    version_byte: u8,
) -> Result<String, WalletError> {
    let pub_bytes = pub_key.serialize();
    let hash = hash160(&pub_bytes);
    let mut data = Vec::with_capacity(21);
    data.push(version_byte);
    data.extend_from_slice(&hash);
    Ok(bs58::encode(data).with_check().into_string())
}

/// Compute hash160: RIPEMD160(SHA256(data))
fn hash160(data: &[u8]) -> [u8; 20] {
    // SHA256
    let sha = Sha256::digest(data);

    // RIPEMD160
    let mut hasher = Ripemd160::new();
    hasher.update(&sha);
    let ripemd = hasher.finalize();

    let mut result = [0u8; 20];
    result.copy_from_slice(&ripemd[..]);
    result
}

#[cfg(test)]
mod tests {
    use super::{
        encode_btc_wif, Network, BITCOIN_WIF_VERSION_MAINNET, BITCOIN_WIF_VERSION_TESTNET,
    };

    #[test]
    fn encode_btc_wif_mainnet_uses_expected_version_byte() {
        let private_key = [1u8; 32];
        let encoded = encode_btc_wif(&private_key, Network::Mainnet).expect("mainnet wif");
        let decoded = bs58::decode(encoded)
            .with_check(None)
            .into_vec()
            .expect("decode mainnet wif");

        assert_eq!(decoded[0], BITCOIN_WIF_VERSION_MAINNET);
        assert_eq!(decoded.len(), 34);
        assert_eq!(decoded[33], 0x01);
    }

    #[test]
    fn encode_btc_wif_testnet_uses_expected_version_byte() {
        let private_key = [2u8; 32];
        let encoded = encode_btc_wif(&private_key, Network::Testnet).expect("testnet wif");
        let decoded = bs58::decode(encoded)
            .with_check(None)
            .into_vec()
            .expect("decode testnet wif");

        assert_eq!(decoded[0], BITCOIN_WIF_VERSION_TESTNET);
        assert_eq!(decoded.len(), 34);
        assert_eq!(decoded[33], 0x01);
    }
}
