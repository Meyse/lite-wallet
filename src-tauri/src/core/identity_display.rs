pub(crate) fn normalize_non_empty(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(trimmed.to_string())
}

pub(crate) fn ensure_identity_handle_suffix(value: &str) -> Option<String> {
    let normalized = normalize_non_empty(value)?;
    if normalized.ends_with('@') {
        return Some(normalized);
    }
    Some(format!("{normalized}@"))
}

fn looks_like_identity_system_suffix(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit())
}

pub(crate) fn format_fully_qualified_name_for_display(raw_fqn: &str) -> Option<String> {
    let with_at = ensure_identity_handle_suffix(raw_fqn)?;
    let without_at = with_at.trim_end_matches('@');

    let Some(last_dot_index) = without_at.rfind('.') else {
        return Some(with_at);
    };
    if last_dot_index == 0 {
        return Some(with_at);
    }

    let suffix = &without_at[last_dot_index + 1..];
    if !looks_like_identity_system_suffix(suffix) {
        return Some(with_at);
    }

    let without_system = without_at[..last_dot_index].trim();
    if without_system.is_empty() {
        return Some(with_at);
    }

    Some(format!("{without_system}@"))
}

pub(crate) fn format_identity_display_name(
    fully_qualified_name: Option<&str>,
    name: Option<&str>,
) -> Option<String> {
    if let Some(display_name) =
        fully_qualified_name.and_then(format_fully_qualified_name_for_display)
    {
        return Some(display_name);
    }

    name.and_then(ensure_identity_handle_suffix)
}

pub(crate) fn format_identity_display_name_or_fallback(
    fully_qualified_name: Option<&str>,
    name: Option<&str>,
    fallback_address: &str,
) -> String {
    format_identity_display_name(fully_qualified_name, name)
        .unwrap_or_else(|| fallback_address.to_string())
}

#[cfg(test)]
mod tests {
    use super::{
        format_fully_qualified_name_for_display, format_identity_display_name,
        format_identity_display_name_or_fallback,
    };

    #[test]
    fn formats_identity_labels_consistently_for_wallet_and_identity_views() {
        assert_eq!(
            format_fully_qualified_name_for_display("alice.VETH"),
            Some("alice@".to_string())
        );
        assert_eq!(
            format_fully_qualified_name_for_display("alice"),
            Some("alice@".to_string())
        );
        assert_eq!(
            format_fully_qualified_name_for_display("alice.vEth"),
            Some("alice.vEth@".to_string())
        );
    }

    #[test]
    fn falls_back_to_identity_address_when_no_label_fields_are_present() {
        assert_eq!(
            format_identity_display_name(Some("alice.VETH"), Some("ignored")),
            Some("alice@".to_string())
        );
        assert_eq!(
            format_identity_display_name(None, Some("alice")),
            Some("alice@".to_string())
        );
        assert_eq!(
            format_identity_display_name_or_fallback(None, None, "iAddressFallback"),
            "iAddressFallback".to_string()
        );
    }
}
