//! Compact v2 cloud identifiers. Installation, lease, claim and explicit request keys have separate contracts.
use rand::distr::{Alphanumeric, SampleString};

const ALPHABET: &[u8; 62] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
const LENGTH: usize = 12;
const WORKSPACE_PREFIX: &[u8; 2] = b"WS";

pub(super) fn random_id() -> String {
    Alphanumeric.sample_string(&mut rand::rng(), LENGTH)
}

fn is_compact(value: &str) -> bool {
    value.len() == LENGTH && value.bytes().all(|c| c.is_ascii_alphanumeric())
}

pub(super) fn valid_token(value: &str) -> bool {
    is_compact(value)
}

pub(super) fn valid_incarnation(value: &str) -> bool {
    is_compact(value)
}

pub(super) fn valid_workspace_id(value: &str) -> bool {
    is_compact(value) && value.as_bytes().starts_with(WORKSPACE_PREFIX)
}

pub(super) fn workspace_from_digest(digest: &[u8; 32]) -> String {
    let mut number = u64::from_be_bytes(digest[..8].try_into().unwrap());
    let mut output = [b'0'; LENGTH];
    output[..2].copy_from_slice(WORKSPACE_PREFIX);
    for digit in output[2..].iter_mut().rev() {
        *digit = ALPHABET[(number % 62) as usize];
        number /= 62;
    }
    String::from_utf8(output.into()).expect("ASCII alphabet")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compact_identifiers_require_exact_ascii_shape() {
        assert!(valid_token(&random_id()));
        assert!(valid_workspace_id("WSAaBb123456"));
        for invalid in ["wsAaBb123456", "WS000000000", "WS00000000000", "WS000000000_", "WS00000000é", "ws_g1IMogNN8O8DXrbzbvh4TOT3_Oxzn-06yjXb6OTskPI"] {
            assert!(!valid_workspace_id(invalid), "{invalid}");
        }
        for invalid in ["550e8400-e29b-41d4-a716-446655440000", "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA", "0123456789_-", "01234567890", "0123456789012", "~human"] {
            assert!(!valid_token(invalid), "{invalid}");
            assert!(!valid_incarnation(invalid), "{invalid}");
        }
    }
}
