//
// Cryptographic operations module
// Security: Implements Verus-Mobile v1 key derivation with secure memory handling
// Last Updated: Tests extended for btc_address consistency and format

pub mod eth_keys;
pub mod key_derivation_v1;
pub mod verus_id_signature;
pub mod wif_encoding;

pub use key_derivation_v1::{derive_keys_from_material, derive_keys_v1};
pub use wif_encoding::Network;

#[cfg(test)]
mod tests {
    use super::{derive_keys_from_material, derive_keys_v1, Network};
    use crate::types::wallet::WalletSecretKind;
    use bs58;

    /// Test that key derivation produces consistent results
    /// TODO: Add known test vectors from Verus-Mobile for exact parity verification
    #[test]
    fn test_key_derivation_consistency() {
        let seed = "test seed phrase for wallet derivation testing purposes";
        let network = Network::Mainnet;

        let keys1 = derive_keys_v1(seed, network).expect("First derivation should succeed");
        let keys2 = derive_keys_v1(seed, network).expect("Second derivation should succeed");

        // Same seed should produce same keys
        assert_eq!(keys1.wif, keys2.wif);
        assert_eq!(keys1.address, keys2.address);
        assert_eq!(keys1.eth_address, keys2.eth_address);
        assert_eq!(keys1.btc_address, keys2.btc_address);
    }

    /// Test that different seeds produce different keys
    #[test]
    fn test_key_derivation_uniqueness() {
        let seed1 = "first test seed phrase";
        let seed2 = "second test seed phrase";
        let network = Network::Mainnet;

        let keys1 = derive_keys_v1(seed1, network).expect("First derivation should succeed");
        let keys2 = derive_keys_v1(seed2, network).expect("Second derivation should succeed");

        // Different seeds should produce different keys
        assert_ne!(keys1.wif, keys2.wif);
        assert_ne!(keys1.address, keys2.address);
        assert_ne!(keys1.eth_address, keys2.eth_address);
        assert_ne!(keys1.btc_address, keys2.btc_address);
    }

    /// Bitcoin P2PKH address should start with "1" and be base58
    #[test]
    fn test_bitcoin_address_format() {
        let seed = "test seed for bitcoin address format";
        let keys = derive_keys_v1(seed, Network::Mainnet).expect("Derivation should succeed");
        assert!(
            keys.btc_address.starts_with('1'),
            "Bitcoin mainnet P2PKH should start with '1'"
        );
        assert!(keys.btc_address.len() >= 33 && keys.btc_address.len() <= 35);
    }

    /// Verus testnet transparent address remains an R-address (same version byte as mainnet).
    #[test]
    fn test_verus_testnet_address_format() {
        let seed = "test seed for verus testnet address format";
        let keys = derive_keys_v1(seed, Network::Testnet).expect("Derivation should succeed");
        assert!(
            keys.address.starts_with('R'),
            "Verus testnet address should start with 'R'"
        );

        let decoded = bs58::decode(&keys.address)
            .with_check(None)
            .into_vec()
            .expect("Address should decode");
        assert_eq!(
            decoded[0], 0x3C,
            "Verus testnet P2PKH version should be 0x3C"
        );
    }

    /// Bitcoin testnet address should keep m/n prefix and differ from Verus testnet address.
    #[test]
    fn test_bitcoin_testnet_address_format() {
        let seed = "test seed for bitcoin testnet address format";
        let keys = derive_keys_v1(seed, Network::Testnet).expect("Derivation should succeed");
        let first = keys
            .btc_address
            .chars()
            .next()
            .expect("Bitcoin testnet address should not be empty");
        assert!(
            first == 'm' || first == 'n',
            "Bitcoin testnet P2PKH should start with 'm' or 'n'"
        );

        let decoded = bs58::decode(&keys.btc_address)
            .with_check(None)
            .into_vec()
            .expect("Address should decode");
        assert_eq!(
            decoded[0], 0x6F,
            "Bitcoin testnet P2PKH version should be 0x6F"
        );
        assert_ne!(
            keys.address, keys.btc_address,
            "Verus and Bitcoin testnet receive addresses should not be identical"
        );
    }

    #[test]
    fn test_derive_keys_from_wif_material() {
        let seed = "material-wif-test-seed";
        let from_seed =
            derive_keys_v1(seed, Network::Mainnet).expect("Seed derivation should succeed");
        let from_wif =
            derive_keys_from_material(&from_seed.wif, WalletSecretKind::Wif, Network::Mainnet)
                .expect("WIF material derivation should succeed");

        assert_eq!(from_wif.wif, from_seed.wif);
        assert_eq!(from_wif.address, from_seed.address);
        assert_eq!(from_wif.eth_address, from_seed.eth_address);
        assert_eq!(from_wif.btc_address, from_seed.btc_address);
    }

    #[test]
    fn test_derive_keys_from_private_hex_material() {
        let seed = "material-private-hex-test-seed";
        let from_seed =
            derive_keys_v1(seed, Network::Mainnet).expect("Seed derivation should succeed");
        let from_hex = derive_keys_from_material(
            &format!("0x{}", from_seed.eth_private_key),
            WalletSecretKind::PrivateKeyHex,
            Network::Mainnet,
        )
        .expect("Private key hex derivation should succeed");

        assert_eq!(from_hex.wif, from_seed.wif);
        assert_eq!(from_hex.address, from_seed.address);
        assert_eq!(from_hex.eth_address, from_seed.eth_address);
        assert_eq!(from_hex.btc_address, from_seed.btc_address);
    }

    #[test]
    fn test_derive_keys_from_seed_text_material() {
        let seed = "custom free-form seed text fallback";
        let from_seed =
            derive_keys_v1(seed, Network::Mainnet).expect("Seed derivation should succeed");
        let from_material =
            derive_keys_from_material(seed, WalletSecretKind::SeedText, Network::Mainnet)
                .expect("Seed text material derivation should succeed");

        assert_eq!(from_material.wif, from_seed.wif);
        assert_eq!(from_material.address, from_seed.address);
        assert_eq!(from_material.eth_address, from_seed.eth_address);
        assert_eq!(from_material.btc_address, from_seed.btc_address);
    }

    // TODO: Test Verus-Mobile v1 parity (VRSC, ETH, BTC)
    // Add known test vectors from Verus-Mobile newsend3 branch:
    // - Seed phrase
    // - Expected WIF
    // - Expected VRSC address
    // - Expected ETH address
    // - Expected BTC P2PKH address (mainnet, version 0x00)
    //
    // Example format:
    // #[test]
    // fn test_verus_mobile_v1_parity() {
    //     let seed = "known seed from Verus-Mobile";
    //     let keys = derive_keys_v1(seed, Network::Mainnet).unwrap();
    //     assert_eq!(keys.address, "expected_vrsc_address");
    //     assert_eq!(keys.eth_address, "expected_eth_address");
    //     assert_eq!(keys.btc_address, "expected_btc_address");
    // }
}
