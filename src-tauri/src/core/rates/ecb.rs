//
// ECB fiat exchange rates parser.
// Converts EUR-based reference rates into USD-based multipliers to match valu-mobile behavior.

use std::collections::HashMap;

use reqwest::Client;

const ECB_RATES_URL: &str = "https://www.ecb.europa.eu/stats/eurofxref/eurofxref-daily.xml";
pub const USD: &str = "USD";
pub const EUR: &str = "EUR";

#[derive(Clone, Debug)]
pub struct EcbReferenceRates {
    pub rates: HashMap<String, f64>,
    pub published_on: Option<String>,
}

pub async fn fetch_usd_reference_rates(client: &Client) -> Result<HashMap<String, f64>, String> {
    fetch_usd_reference_snapshot(client)
        .await
        .map(|snapshot| snapshot.rates)
}

pub async fn fetch_usd_reference_snapshot(client: &Client) -> Result<EcbReferenceRates, String> {
    let response = client
        .get(ECB_RATES_URL)
        .send()
        .await
        .map_err(|e| format!("ecb request failed: {}", e))?;
    if !response.status().is_success() {
        return Err(format!("ecb returned HTTP {}", response.status()));
    }

    let xml = response
        .text()
        .await
        .map_err(|e| format!("ecb response decode failed: {}", e))?;

    Ok(parse_usd_reference_snapshot_from_xml(&xml))
}

/// Builds fiat prices for a coin given its USD price and USD-relative fiat multipliers.
pub fn build_coin_fiat_rates(
    usd_price: f64,
    usd_reference_rates: &HashMap<String, f64>,
) -> HashMap<String, f64> {
    let mut out = HashMap::new();
    if !(usd_price.is_finite() && usd_price > 0.0) {
        return out;
    }

    out.insert(USD.to_string(), usd_price);

    for (fiat, multiplier) in usd_reference_rates {
        if fiat == USD {
            continue;
        }
        if multiplier.is_finite() && *multiplier > 0.0 {
            out.insert(fiat.clone(), usd_price * multiplier);
        }
    }

    out
}

pub fn parse_usd_reference_rates_from_xml(xml: &str) -> HashMap<String, f64> {
    parse_usd_reference_snapshot_from_xml(xml).rates
}

pub fn parse_usd_reference_snapshot_from_xml(xml: &str) -> EcbReferenceRates {
    let mut eur_reference_rates = HashMap::<String, f64>::new();
    let mut published_on = None;

    for line in xml.lines() {
        if published_on.is_none() && line.contains("time=") {
            published_on = extract_attr(line, "time").filter(|value| !value.trim().is_empty());
        }
        if !line.contains("currency=") || !line.contains("rate=") {
            continue;
        }

        let Some(currency) = extract_attr(line, "currency") else {
            continue;
        };
        let Some(rate_raw) = extract_attr(line, "rate") else {
            continue;
        };

        let currency = currency.trim().to_uppercase();
        if currency.is_empty() || !currency.chars().all(|c| c.is_ascii_alphabetic()) {
            continue;
        }

        let Ok(rate) = rate_raw.trim().parse::<f64>() else {
            continue;
        };

        if rate.is_finite() && rate > 0.0 {
            eur_reference_rates.insert(currency, rate);
        }
    }

    // ECB rates are EUR-relative. Mobile explicitly sets EUR=1.
    eur_reference_rates.insert(EUR.to_string(), 1.0);

    let usd_per_eur = eur_reference_rates.get(USD).copied().filter(|v| *v > 0.0);
    let mut usd_reference_rates = HashMap::<String, f64>::new();

    if let Some(usd_rate) = usd_per_eur {
        for (currency, eur_rate) in eur_reference_rates {
            if currency == USD {
                continue;
            }
            if eur_rate.is_finite() && eur_rate > 0.0 {
                usd_reference_rates.insert(currency, eur_rate / usd_rate);
            }
        }
    }

    // Mobile hard-sets USD to 1 after normalization.
    usd_reference_rates.insert(USD.to_string(), 1.0);
    EcbReferenceRates {
        rates: usd_reference_rates,
        published_on,
    }
}

fn extract_attr(line: &str, attr: &str) -> Option<String> {
    let single = format!("{attr}='");
    if let Some(start) = line.find(&single) {
        let value_start = start + single.len();
        let value_end = value_start + line[value_start..].find('\'')?;
        return Some(line[value_start..value_end].to_string());
    }

    let double = format!("{attr}=\"");
    if let Some(start) = line.find(&double) {
        let value_start = start + double.len();
        let value_end = value_start + line[value_start..].find('"')?;
        return Some(line[value_start..value_end].to_string());
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_usd_reference_rates_matches_expected_math() {
        let xml = r#"
            <Cube time='2026-02-13'>
              <Cube currency='USD' rate='1.1862'/>
              <Cube currency='JPY' rate='176.01'/>
              <Cube currency='GBP' rate='0.855'/>
            </Cube>
        "#;

        let rates = parse_usd_reference_rates_from_xml(xml);
        assert_eq!(rates.get("USD"), Some(&1.0));
        assert_eq!(rates.get("EUR"), Some(&(1.0 / 1.1862)));
        assert_eq!(rates.get("JPY"), Some(&(176.01 / 1.1862)));
        assert_eq!(rates.get("GBP"), Some(&(0.855 / 1.1862)));
    }

    #[test]
    fn parse_snapshot_preserves_ecb_publication_date() {
        let xml = r#"
            <Cube time='2026-09-04'>
              <Cube currency='USD' rate='1.16'/>
            </Cube>
        "#;

        let snapshot = parse_usd_reference_snapshot_from_xml(xml);
        assert_eq!(snapshot.published_on.as_deref(), Some("2026-09-04"));
        assert_eq!(snapshot.rates.get("USD"), Some(&1.0));
    }

    #[test]
    fn build_coin_fiat_rates_multiplies_reference_rates() {
        let refs = HashMap::from([
            ("USD".to_string(), 1.0),
            ("EUR".to_string(), 0.9),
            ("JPY".to_string(), 150.0),
        ]);
        let built = build_coin_fiat_rates(2.5, &refs);

        assert_eq!(built.get("USD"), Some(&2.5));
        assert_eq!(built.get("EUR"), Some(&2.25));
        assert_eq!(built.get("JPY"), Some(&375.0));
    }
}
