//
// Public fiat-rate cache shared by update-engine runs.
// Contains fetched market data only; no account, balance, or signing state.

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::core::coins::{CoinDefinition, Protocol};
use crate::types::wallet::WalletNetwork;

pub const MARKET_RATE_REFRESH_SECS: u64 = crate::core::updates::RATES_REFRESH_SECS;
pub const MARKET_RATE_DISPLAY_MAX_AGE_SECS: u64 = 15 * 60;
pub const ECB_REFERENCE_REFRESH_SECS: u64 = 6 * 60 * 60;
pub const ECB_REFERENCE_MAX_AGE_SECS: u64 = 7 * 24 * 60 * 60;
pub const ECB_PUBLICATION_MAX_AGE_DAYS: u64 = 5;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PublicRateSource {
    CoinPaprika,
    Pbaas,
    BridgeVeth,
    StrictAlias { counterpart_coin_id: String },
}

impl PublicRateSource {
    pub fn label(&self) -> &'static str {
        match self {
            Self::CoinPaprika => "coinpaprika",
            Self::Pbaas => "pbaas",
            Self::BridgeVeth => "bridge_veth",
            Self::StrictAlias { .. } => "strict_alias",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PublicRateIdentity {
    pub network: WalletNetwork,
    pub coin_id: String,
    pub currency_id: String,
    pub system_id: String,
    pub protocol: String,
    pub coin_paprika_id: Option<String>,
}

impl PublicRateIdentity {
    pub fn for_coin(network: WalletNetwork, coin: &CoinDefinition) -> Self {
        Self {
            network,
            coin_id: normalize_identity_part(&coin.id),
            currency_id: normalize_identity_part(&coin.currency_id),
            system_id: normalize_identity_part(&coin.system_id),
            protocol: match coin.proto {
                Protocol::Vrsc => "vrsc",
                Protocol::Btc => "btc",
                Protocol::Eth => "eth",
                Protocol::Erc20 => "erc20",
            }
            .to_string(),
            coin_paprika_id: coin
                .coin_paprika_id
                .as_deref()
                .map(normalize_identity_part)
                .filter(|value| !value.is_empty()),
        }
    }
}

#[derive(Clone, Debug)]
pub struct CachedMarketRates {
    pub identity: PublicRateIdentity,
    pub rates: HashMap<String, f64>,
    pub usd_price: f64,
    pub usd_change_24h_pct: Option<f64>,
    pub source: PublicRateSource,
    pub fetched_at_unix_secs: u64,
}

impl CachedMarketRates {
    pub fn age_secs(&self, now_unix_secs: u64) -> u64 {
        now_unix_secs.saturating_sub(self.fetched_at_unix_secs)
    }

    pub fn is_displayable(&self, now_unix_secs: u64) -> bool {
        self.fetched_at_unix_secs <= now_unix_secs
            && self.age_secs(now_unix_secs) <= MARKET_RATE_DISPLAY_MAX_AGE_SECS
    }

    pub fn needs_refresh(&self, now_unix_secs: u64) -> bool {
        self.fetched_at_unix_secs > now_unix_secs
            || self.age_secs(now_unix_secs) >= MARKET_RATE_REFRESH_SECS
    }
}

#[derive(Clone, Debug)]
pub struct CachedEcbReferenceRates {
    pub rates: HashMap<String, f64>,
    pub published_on: Option<String>,
    pub fetched_at_unix_secs: u64,
}

impl CachedEcbReferenceRates {
    pub fn age_secs(&self, now_unix_secs: u64) -> u64 {
        now_unix_secs.saturating_sub(self.fetched_at_unix_secs)
    }

    pub fn is_usable(&self, now_unix_secs: u64) -> bool {
        self.has_valid_reference_rates()
            && self.fetched_at_unix_secs <= now_unix_secs
            && self.age_secs(now_unix_secs) <= ECB_REFERENCE_MAX_AGE_SECS
            && self
                .publication_age_days(now_unix_secs)
                .is_some_and(|age_days| age_days <= ECB_PUBLICATION_MAX_AGE_DAYS)
    }

    pub fn needs_refresh(&self, now_unix_secs: u64) -> bool {
        !self.is_usable(now_unix_secs) || self.age_secs(now_unix_secs) >= ECB_REFERENCE_REFRESH_SECS
    }

    pub fn publication_age_days(&self, now_unix_secs: u64) -> Option<u64> {
        let published_day = parse_iso_date_to_unix_day(self.published_on.as_deref()?)?;
        let current_day = i64::try_from(now_unix_secs / (24 * 60 * 60)).ok()?;
        u64::try_from(current_day.checked_sub(published_day)?).ok()
    }

    fn has_valid_reference_rates(&self) -> bool {
        self.rates
            .get("USD")
            .is_some_and(|value| value.is_finite() && *value > 0.0)
            && self
                .rates
                .iter()
                .any(|(currency, value)| currency != "USD" && value.is_finite() && *value > 0.0)
    }
}

#[derive(Debug, Default)]
pub struct PublicRatesCache {
    market: HashMap<PublicRateIdentity, CachedMarketRates>,
    ecb: Option<CachedEcbReferenceRates>,
}

impl PublicRatesCache {
    pub fn market_rate(
        &self,
        network: WalletNetwork,
        coin: &CoinDefinition,
    ) -> Option<&CachedMarketRates> {
        self.market
            .get(&PublicRateIdentity::for_coin(network, coin))
    }

    pub fn displayable_market_rates(
        &self,
        network: WalletNetwork,
        coins: &[CoinDefinition],
        now_unix_secs: u64,
    ) -> Vec<CachedMarketRates> {
        coins
            .iter()
            .filter_map(|coin| self.market_rate(network, coin))
            .filter(|entry| entry.is_displayable(now_unix_secs))
            .cloned()
            .collect()
    }

    pub fn store_market_rate(&mut self, entry: CachedMarketRates) {
        self.market.insert(entry.identity.clone(), entry);
    }

    pub fn usable_ecb_reference_rates(
        &self,
        now_unix_secs: u64,
    ) -> Option<CachedEcbReferenceRates> {
        self.ecb
            .as_ref()
            .filter(|entry| entry.is_usable(now_unix_secs))
            .cloned()
    }

    pub fn ecb_needs_refresh(&self, now_unix_secs: u64) -> bool {
        self.ecb
            .as_ref()
            .map_or(true, |entry| entry.needs_refresh(now_unix_secs))
    }

    pub fn store_ecb_reference_rates(&mut self, entry: CachedEcbReferenceRates) {
        self.ecb = Some(entry);
    }
}

pub fn unix_timestamp_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn normalize_identity_part(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

fn parse_iso_date_to_unix_day(value: &str) -> Option<i64> {
    let bytes = value.as_bytes();
    if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
        return None;
    }
    if !bytes
        .iter()
        .enumerate()
        .all(|(index, byte)| matches!(index, 4 | 7) || byte.is_ascii_digit())
    {
        return None;
    }

    let year = parse_decimal(&bytes[0..4])? as i64;
    let month = parse_decimal(&bytes[5..7])?;
    let day = parse_decimal(&bytes[8..10])?;
    if year < 1970 || !(1..=12).contains(&month) {
        return None;
    }
    let leap_year = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let max_day = match month {
        2 if leap_year => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    };
    if !(1..=max_day).contains(&day) {
        return None;
    }

    // Howard Hinnant's civil-date conversion, offset to Unix epoch day zero.
    let adjusted_year = year - i64::from(month <= 2);
    let era = adjusted_year.div_euclid(400);
    let year_of_era = adjusted_year - era * 400;
    let shifted_month = i64::from(month) + if month > 2 { -3 } else { 9 };
    let day_of_year = (153 * shifted_month + 2) / 5 + i64::from(day) - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    Some(era * 146_097 + day_of_era - 719_468)
}

fn parse_decimal(bytes: &[u8]) -> Option<u32> {
    bytes.iter().try_fold(0_u32, |value, byte| {
        value
            .checked_mul(10)?
            .checked_add(u32::from(byte.checked_sub(b'0')?))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::coins::{Channel, Protocol};

    fn sample_coin(id: &str, currency_id: &str, system_id: &str) -> CoinDefinition {
        CoinDefinition {
            id: id.to_string(),
            currency_id: currency_id.to_string(),
            system_id: system_id.to_string(),
            display_ticker: id.to_string(),
            display_name: id.to_string(),
            coin_paprika_id: None,
            proto: Protocol::Vrsc,
            compatible_channels: vec![Channel::Vrpc],
            decimals: 8,
            vrpc_endpoints: vec![],
            dlight_endpoints: None,
            electrum_endpoints: None,
            seconds_per_block: 60,
            mapped_to: None,
            is_testnet: false,
        }
    }

    fn market_entry(
        network: WalletNetwork,
        coin: &CoinDefinition,
        fetched_at_unix_secs: u64,
    ) -> CachedMarketRates {
        CachedMarketRates {
            identity: PublicRateIdentity::for_coin(network, coin),
            rates: HashMap::from([("USD".to_string(), 1.0)]),
            usd_price: 1.0,
            usd_change_24h_pct: None,
            source: PublicRateSource::CoinPaprika,
            fetched_at_unix_secs,
        }
    }

    #[test]
    fn market_cache_requires_exact_network_currency_and_source_identity() {
        let coin = sample_coin("TOKEN", "iCurrencyA", "iSystem");
        let changed_currency = sample_coin("TOKEN", "iCurrencyB", "iSystem");
        let mut changed_source = coin.clone();
        changed_source.coin_paprika_id = Some("token-new-source".to_string());
        let mut changed_protocol = coin.clone();
        changed_protocol.proto = Protocol::Erc20;
        let mut cache = PublicRatesCache::default();
        cache.store_market_rate(market_entry(WalletNetwork::Mainnet, &coin, 1_000));

        assert!(cache.market_rate(WalletNetwork::Mainnet, &coin).is_some());
        assert!(cache.market_rate(WalletNetwork::Testnet, &coin).is_none());
        assert!(cache
            .market_rate(WalletNetwork::Mainnet, &changed_currency)
            .is_none());
        assert!(cache
            .market_rate(WalletNetwork::Mainnet, &changed_source)
            .is_none());
        assert!(cache
            .market_rate(WalletNetwork::Mainnet, &changed_protocol)
            .is_none());
    }

    #[test]
    fn market_cache_serves_stale_while_refresh_then_expires() {
        let coin = sample_coin("VRSC", "VRSC", "iSystem");
        let entry = market_entry(WalletNetwork::Mainnet, &coin, 10_000);

        assert!(!entry.needs_refresh(10_000 + MARKET_RATE_REFRESH_SECS - 1));
        assert!(entry.needs_refresh(10_000 + MARKET_RATE_REFRESH_SECS));
        assert!(entry.is_displayable(10_000 + MARKET_RATE_DISPLAY_MAX_AGE_SECS));
        assert!(!entry.is_displayable(10_000 + MARKET_RATE_DISPLAY_MAX_AGE_SECS + 1));
        assert!(!entry.is_displayable(9_999));
        assert!(entry.needs_refresh(9_999));
    }

    fn unix_at_noon(date: &str) -> u64 {
        u64::try_from(parse_iso_date_to_unix_day(date).expect("valid test date"))
            .expect("positive test date")
            * 24
            * 60
            * 60
            + 12 * 60 * 60
    }

    #[test]
    fn ecb_cache_allows_weekend_and_holiday_grace_but_bounds_publication_age() {
        let entry = CachedEcbReferenceRates {
            rates: HashMap::from([("USD".to_string(), 1.0), ("EUR".to_string(), 0.9)]),
            published_on: Some("2026-09-04".to_string()),
            fetched_at_unix_secs: unix_at_noon("2026-09-09"),
        };

        assert!(entry.is_usable(unix_at_noon("2026-09-09")));
        assert!(!entry.is_usable(unix_at_noon("2026-09-10")));
        assert!(entry.needs_refresh(unix_at_noon("2026-09-10")));
    }

    #[test]
    fn ecb_cache_rejects_missing_invalid_or_future_publication_dates() {
        for published_on in [
            None,
            Some("not-a-date".to_string()),
            Some("2026-02-30".to_string()),
            Some("2026-09-10".to_string()),
        ] {
            let entry = CachedEcbReferenceRates {
                rates: HashMap::from([("USD".to_string(), 1.0), ("EUR".to_string(), 0.9)]),
                published_on,
                fetched_at_unix_secs: unix_at_noon("2026-09-09"),
            };

            assert!(!entry.is_usable(unix_at_noon("2026-09-09")));
            assert!(entry.needs_refresh(unix_at_noon("2026-09-09")));
        }

        let future_fetch = CachedEcbReferenceRates {
            rates: HashMap::from([("USD".to_string(), 1.0), ("EUR".to_string(), 0.9)]),
            published_on: Some("2026-09-09".to_string()),
            fetched_at_unix_secs: unix_at_noon("2026-09-10"),
        };
        assert!(!future_fetch.is_usable(unix_at_noon("2026-09-09")));
    }

    #[test]
    fn ecb_cache_rejects_a_snapshot_without_reference_currencies() {
        let entry = CachedEcbReferenceRates {
            rates: HashMap::from([("USD".to_string(), 1.0)]),
            published_on: Some("2026-09-09".to_string()),
            fetched_at_unix_secs: unix_at_noon("2026-09-09"),
        };

        assert!(!entry.is_usable(unix_at_noon("2026-09-09")));
    }

    #[test]
    fn iso_date_parser_uses_unix_epoch_days() {
        assert_eq!(parse_iso_date_to_unix_day("1970-01-01"), Some(0));
        assert_eq!(parse_iso_date_to_unix_day("1970-01-02"), Some(1));
        assert_eq!(parse_iso_date_to_unix_day("2000-02-29"), Some(11_016));
    }
}
