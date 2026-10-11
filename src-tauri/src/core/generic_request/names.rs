//! Address encoding and VerusID name hashing, as verus-typescript-primitives
//! does them (`utils/address.ts`).

use ripemd::Ripemd160;
use sha2::{Digest, Sha256};
use zcash_client_backend::encoding::{decode_payment_address, encode_payment_address};
use zcash_protocol::constants::mainnet;

pub(crate) const I_ADDRESS_VERSION: u8 = 102;
pub(crate) const X_ADDRESS_VERSION: u8 = 137;
pub(crate) const R_ADDRESS_VERSION: u8 = 60;
pub(crate) const P2SH_ADDRESS_VERSION: u8 = 85;
pub(crate) const MAINNET_SYSTEM_ID: &str = "i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV";
pub(crate) const TESTNET_SYSTEM_ID: &str = "iJhCezBExJHvtyH3fGhNnt2NhU4Ztkf2yq";

pub(crate) fn sha256d(parts: &[&[u8]]) -> [u8; 32] {
    let mut first = Sha256::new();
    for part in parts {
        first.update(part);
    }
    Sha256::digest(first.finalize()).into()
}

pub(crate) fn sha256(data: &[u8]) -> [u8; 32] {
    Sha256::digest(data).into()
}

pub(crate) fn hash160(data: &[u8]) -> [u8; 20] {
    Ripemd160::digest(Sha256::digest(data)).into()
}

pub(crate) fn base58_check(version: u8, hash: &[u8; 20]) -> String {
    let mut payload = Vec::with_capacity(21);
    payload.push(version);
    payload.extend_from_slice(hash);
    bs58::encode(payload).with_check().into_string()
}

pub(crate) fn i_address(hash: &[u8; 20]) -> String {
    base58_check(I_ADDRESS_VERSION, hash)
}

/// The version byte and hash of a base58check address.
pub(crate) fn decode_base58_check(address: &str) -> Option<(u8, [u8; 20])> {
    let decoded = bs58::decode(address.trim())
        .with_check(None)
        .into_vec()
        .ok()?;
    if decoded.len() != 21 {
        return None;
    }
    Some((decoded[0], decoded[1..].try_into().ok()?))
}

pub(crate) fn decode_i_address(address: &str) -> Option<[u8; 20]> {
    match decode_base58_check(address)? {
        (I_ADDRESS_VERSION, hash) => Some(hash),
        _ => None,
    }
}

/// Encodes a Sapling payment address (`d` then `pk_d`). Verus uses the `zs`
/// prefix on mainnet and testnet.
pub(crate) fn sapling_address(raw: &[u8; 43]) -> Option<String> {
    let address = sapling::PaymentAddress::from_bytes(raw)?;
    Some(encode_payment_address(
        mainnet::HRP_SAPLING_PAYMENT_ADDRESS,
        &address,
    ))
}

pub(crate) fn decode_sapling_address(address: &str) -> Option<[u8; 43]> {
    let decoded = decode_payment_address(
        mainnet::HRP_SAPLING_PAYMENT_ADDRESS,
        address.trim().to_ascii_lowercase().as_str(),
    )
    .ok()?;
    Some(decoded.to_bytes())
}

pub(crate) fn root_system_name(is_testnet: bool) -> &'static str {
    if is_testnet {
        "VRSCTEST"
    } else {
        "VRSC"
    }
}

pub(crate) fn root_system_id(is_testnet: bool) -> &'static str {
    if is_testnet {
        TESTNET_SYSTEM_ID
    } else {
        MAINNET_SYSTEM_ID
    }
}

/// `nameAndParentAddrToIAddr`: the identity hash of one name under a parent.
pub(crate) fn name_id(name: &str, parent: Option<&[u8; 20]>) -> [u8; 20] {
    let id_hash = sha256d(&[name.to_ascii_lowercase().as_bytes()]);
    let id_hash = match parent {
        Some(parent) => sha256d(&[parent, &id_hash]),
        None => id_hash,
    };
    hash160(&id_hash)
}

/// `fqnToAddress`: the root system is appended unless the name already ends
/// with it, or with a trailing dot that marks an absolute name.
pub(crate) fn fqn_to_id(fully_qualified_name: &str, root_system_name: &str) -> Option<[u8; 20]> {
    let at_parts = fully_qualified_name.split('@').collect::<Vec<_>>();
    if at_parts.len() > 2
        || at_parts[0].is_empty()
        || (at_parts.len() == 2 && !at_parts[1].is_empty())
    {
        return None;
    }

    let mut components = at_parts[0]
        .split('.')
        .map(|component| component.trim_matches(' ').to_string())
        .collect::<Vec<_>>();
    let last = components.last()?;
    if !last.eq_ignore_ascii_case(root_system_name) && !last.is_empty() {
        components.push(root_system_name.to_string());
    }

    let name = components.remove(0);
    if name.is_empty() {
        return None;
    }

    let mut parent: Option<[u8; 20]> = None;
    for component in components.iter().rev() {
        if component.is_empty() {
            continue;
        }
        parent = Some(name_id(component, parent.as_ref()));
    }

    Some(name_id(&name, parent.as_ref()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fully_qualified_names_hash_like_the_library_and_daemon() {
        // Values from `getidentity` on VRSCTEST.
        assert_eq!(
            i_address(&fqn_to_id("TestBob.antafri@", "VRSCTEST").unwrap()),
            "iGHXn8mvh9kutBNotJ2LCNz3zDxvemEb2i"
        );
        assert_eq!(
            i_address(&fqn_to_id("player7@", "VRSCTEST").unwrap()),
            "i8KEYF3QeR1xPVVCL5BSpDXkA1CAfFDvmk"
        );
        assert_eq!(
            i_address(&fqn_to_id("VRSCTEST@", "VRSCTEST").unwrap()),
            TESTNET_SYSTEM_ID
        );
        assert_eq!(
            i_address(&fqn_to_id("VRSC@", "VRSC").unwrap()),
            MAINNET_SYSTEM_ID
        );
    }

    #[test]
    fn invalid_names_are_rejected() {
        assert!(fqn_to_id("@", "VRSCTEST").is_none());
        assert!(fqn_to_id("a@b@", "VRSCTEST").is_none());
        assert!(fqn_to_id("alice@vrsc", "VRSC").is_none());
    }

    #[test]
    fn sapling_addresses_round_trip() {
        let address =
            "zs13akhfdhypx9a8t2efkfgxpd5d9h3glz7d5pug2ys65mrzt4mw5vjqx0vw29gx82y3e0xjjuhrwn";
        let raw = decode_sapling_address(address).unwrap();
        assert_eq!(sapling_address(&raw).unwrap(), address);
    }
}
