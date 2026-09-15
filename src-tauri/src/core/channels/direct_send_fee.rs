use ethers::types::U256;

use crate::types::transaction::DirectSendFeeMode;
use crate::types::WalletError;

pub(crate) const MIN_EVM_MAX_FEE_PER_GAS_WEI: u64 = 1_000_000_000;
pub(crate) const STANDARD_EVM_HEADROOM_DIVISOR: u64 = 3;
pub(crate) const ECONOMY_PRIORITY_DIVISOR: u64 = 2;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ResolvedEvmFee {
    pub max_fee_per_gas: U256,
    pub max_priority_fee_per_gas: U256,
}

pub(crate) fn resolve_btc_fee_rate(
    mode: DirectSendFeeMode,
    economy_sat_per_vbyte: u64,
    standard_sat_per_vbyte: u64,
    minimum_sat_per_vbyte: u64,
) -> Result<u64, WalletError> {
    if economy_sat_per_vbyte == 0 || standard_sat_per_vbyte == 0 || minimum_sat_per_vbyte == 0 {
        return Err(WalletError::NetworkError);
    }

    Ok(match mode {
        DirectSendFeeMode::Economy => economy_sat_per_vbyte.max(minimum_sat_per_vbyte),
        DirectSendFeeMode::Standard => standard_sat_per_vbyte.max(minimum_sat_per_vbyte),
    })
}

pub(crate) fn resolve_direct_evm_fee(
    mode: DirectSendFeeMode,
    provider_max_fee_per_gas: U256,
    provider_priority_fee_per_gas: U256,
) -> ResolvedEvmFee {
    let floor = U256::from(MIN_EVM_MAX_FEE_PER_GAS_WEI);
    let max_fee_per_gas = match mode {
        DirectSendFeeMode::Economy => provider_max_fee_per_gas,
        DirectSendFeeMode::Standard => provider_max_fee_per_gas
            .saturating_add(provider_max_fee_per_gas / U256::from(STANDARD_EVM_HEADROOM_DIVISOR)),
    }
    .max(floor);

    let requested_priority = match mode {
        DirectSendFeeMode::Economy => {
            provider_priority_fee_per_gas / U256::from(ECONOMY_PRIORITY_DIVISOR)
        }
        DirectSendFeeMode::Standard => provider_priority_fee_per_gas,
    };

    ResolvedEvmFee {
        max_fee_per_gas,
        max_priority_fee_per_gas: requested_priority.min(max_fee_per_gas),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn btc_modes_use_the_expected_quote_and_minimum_clamp() {
        assert_eq!(
            resolve_btc_fee_rate(DirectSendFeeMode::Economy, 1, 8, 3).expect("economy"),
            3
        );
        assert_eq!(
            resolve_btc_fee_rate(DirectSendFeeMode::Standard, 1, 8, 3).expect("standard"),
            8
        );
        assert!(resolve_btc_fee_rate(DirectSendFeeMode::Economy, 0, 8, 3).is_err());
    }

    #[test]
    fn evm_economy_removes_headroom_and_halves_priority() {
        let resolved = resolve_direct_evm_fee(
            DirectSendFeeMode::Economy,
            U256::from(30_000_000_000u64),
            U256::from(4_000_000_000u64),
        );
        assert_eq!(resolved.max_fee_per_gas, U256::from(30_000_000_000u64));
        assert_eq!(
            resolved.max_priority_fee_per_gas,
            U256::from(2_000_000_000u64)
        );
    }

    #[test]
    fn evm_standard_preserves_one_third_headroom_and_clamps_priority() {
        let resolved = resolve_direct_evm_fee(
            DirectSendFeeMode::Standard,
            U256::from(30_000_000_000u64),
            U256::from(50_000_000_000u64),
        );
        assert_eq!(resolved.max_fee_per_gas, U256::from(40_000_000_000u64));
        assert_eq!(resolved.max_priority_fee_per_gas, resolved.max_fee_per_gas);
    }

    #[test]
    fn evm_modes_retain_the_one_gwei_max_fee_floor() {
        for mode in [DirectSendFeeMode::Economy, DirectSendFeeMode::Standard] {
            let resolved = resolve_direct_evm_fee(mode, U256::from(1u64), U256::from(2u64));
            assert_eq!(
                resolved.max_fee_per_gas,
                U256::from(MIN_EVM_MAX_FEE_PER_GAS_WEI)
            );
            assert!(resolved.max_priority_fee_per_gas <= resolved.max_fee_per_gas);
        }
    }

    #[test]
    fn selected_evm_policy_drives_the_maximum_fee_envelope() {
        let economy = resolve_direct_evm_fee(
            DirectSendFeeMode::Economy,
            U256::from(30u64),
            U256::from(4u64),
        );
        let standard = resolve_direct_evm_fee(
            DirectSendFeeMode::Standard,
            U256::from(30u64),
            U256::from(4u64),
        );
        let gas_limit = U256::from(65_000u64);
        assert_eq!(
            gas_limit.saturating_mul(economy.max_fee_per_gas),
            gas_limit.saturating_mul(U256::from(MIN_EVM_MAX_FEE_PER_GAS_WEI))
        );
        assert_eq!(
            gas_limit.saturating_mul(standard.max_fee_per_gas),
            gas_limit.saturating_mul(U256::from(MIN_EVM_MAX_FEE_PER_GAS_WEI))
        );

        let above_floor_economy = resolve_direct_evm_fee(
            DirectSendFeeMode::Economy,
            U256::from(30_000_000_000u64),
            U256::from(4_000_000_000u64),
        );
        let above_floor_standard = resolve_direct_evm_fee(
            DirectSendFeeMode::Standard,
            U256::from(30_000_000_000u64),
            U256::from(4_000_000_000u64),
        );
        assert!(
            gas_limit.saturating_mul(above_floor_standard.max_fee_per_gas)
                > gas_limit.saturating_mul(above_floor_economy.max_fee_per_gas)
        );
    }
}
