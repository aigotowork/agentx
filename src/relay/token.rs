//! Relay join token encode/decode.
//!
//! Join tokens encode a version byte, a 16-byte Relay ID, a 32-byte PSK,
//! and either a one-byte public broker index or a broker URL.

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;

use super::DEFAULT_BROKERS;

const PSK_LEN: usize = 32;
const VERSION: u8 = 0x04;

/// A decoded join token always supplies the key required to join a Relay.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedToken {
    pub relay_id: String,
    pub broker_url: String,
    pub psk: [u8; PSK_LEN],
}

/// Encode relay_id, broker, and PSK into a compact v0x04 join token.
pub fn encode_join_token(relay_id: &str, broker_url: &str, psk: &[u8; PSK_LEN]) -> Option<String> {
    let uuid_bytes = uuid_to_bytes(relay_id)?;

    // Public broker → 1+16+32+1 = 50 bytes; private broker → variable.
    for (i, (host, port)) in DEFAULT_BROKERS.iter().enumerate() {
        let mqtts = format!("mqtts://{}:{}", host, port);
        let mqtt = format!("mqtt://{}:{}", host, port);
        if broker_url == mqtts || broker_url == mqtt {
            let mut buf = Vec::with_capacity(1 + 16 + PSK_LEN + 1);
            buf.push(VERSION);
            buf.extend_from_slice(&uuid_bytes);
            buf.extend_from_slice(psk);
            buf.push(i as u8);
            return Some(URL_SAFE_NO_PAD.encode(&buf));
        }
    }

    let mut buf = Vec::with_capacity(1 + 16 + PSK_LEN + broker_url.len());
    buf.push(VERSION);
    buf.extend_from_slice(&uuid_bytes);
    buf.extend_from_slice(psk);
    buf.extend_from_slice(broker_url.as_bytes());
    Some(URL_SAFE_NO_PAD.encode(&buf))
}

/// Decode a join token. Returns `None` if the token is unparseable.
pub fn decode_join_token(token: &str) -> Option<DecodedToken> {
    let raw = URL_SAFE_NO_PAD.decode(token.trim_end_matches('=')).ok()?;
    if raw.first() != Some(&VERSION) || raw.len() < 17 + PSK_LEN + 1 {
        return None;
    }
    let relay_id = bytes_to_uuid(&raw[1..17])?;
    let psk = raw[17..17 + PSK_LEN].try_into().ok()?;
    // A broker URL needs at least host:port, so a one-byte tail is always an index.
    let broker_url = match &raw[17 + PSK_LEN..] {
        [idx] => {
            let (host, port) = DEFAULT_BROKERS.get(usize::from(*idx))?;
            format!("mqtts://{host}:{port}")
        }
        tail => std::str::from_utf8(tail).ok()?.to_string(),
    };
    Some(DecodedToken { relay_id, broker_url, psk })
}

/// Parse UUID string (with hyphens) to 16 bytes.
fn uuid_to_bytes(uuid_str: &str) -> Option<[u8; 16]> {
    let hex: String = uuid_str.chars().filter(|c| *c != '-').collect();
    if hex.len() != 32 {
        return None;
    }
    let mut bytes = [0u8; 16];
    for i in 0..16 {
        bytes[i] = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).ok()?;
    }
    Some(bytes)
}

/// Format 16 bytes as hyphenated UUID string.
fn bytes_to_uuid(bytes: &[u8]) -> Option<String> {
    if bytes.len() != 16 {
        return None;
    }
    let hex: String = bytes.iter().map(|b| format!("{:02x}", b)).collect();
    Some(format!("{}-{}-{}-{}-{}", &hex[0..8], &hex[8..12], &hex[12..16], &hex[16..20], &hex[20..32]))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_psk() -> [u8; 32] {
        let mut k = [0u8; 32];
        for (i, b) in k.iter_mut().enumerate() {
            *b = (i as u8).wrapping_mul(7);
        }
        k
    }

    #[test]
    fn test_uuid_roundtrip() {
        let uuid = "a1b2c3d4-e5f6-7890-abcd-ef1234567890";
        let bytes = uuid_to_bytes(uuid).unwrap();
        let back = bytes_to_uuid(&bytes).unwrap();
        assert_eq!(uuid, back);
    }

    #[test]
    fn test_uuid_to_bytes_invalid() {
        assert!(uuid_to_bytes("not-a-uuid").is_none());
        assert!(uuid_to_bytes("").is_none());
    }

    #[test]
    fn test_decode_invalid_token() {
        assert!(decode_join_token("").is_none());
        assert!(decode_join_token("short").is_none());
        assert!(decode_join_token("!!!invalid-base64!!!").is_none());
    }

    #[test]
    fn test_decode_v0x04_bad_broker_index() {
        let uuid_bytes = uuid_to_bytes("a1b2c3d4-e5f6-7890-abcd-ef1234567890").unwrap();
        let mut buf = vec![VERSION];
        buf.extend_from_slice(&uuid_bytes);
        buf.extend_from_slice(&fake_psk());
        buf.push(255);
        let token = URL_SAFE_NO_PAD.encode(&buf);
        assert!(decode_join_token(&token).is_none());
    }

    #[test]
    fn test_keyless_tokens_rejected() {
        for token in ["AaGyw9Tl9niQq83vEjRWeJAA", "AqGyw9Tl9niQq83vEjRWeJBtcXR0czovL215LXByaXZhdGUtYnJva2VyLmV4YW1wbGUuY29tOjg4ODM"] {
            assert!(decode_join_token(token).is_none());
        }
    }

    #[test]
    fn test_encode_decode_v0x04_public_broker() {
        let relay_id = "a1b2c3d4-e5f6-7890-abcd-ef1234567890";
        let psk = fake_psk();
        for (i, (host, port)) in DEFAULT_BROKERS.iter().enumerate() {
            let broker = format!("mqtts://{}:{}", host, port);
            let token = encode_join_token(relay_id, &broker, &psk).unwrap();
            let decoded = decode_join_token(&token).unwrap();
            assert_eq!(decoded.relay_id, relay_id, "broker index {}", i);
            assert_eq!(decoded.broker_url, broker);
            assert_eq!(decoded.psk, psk);
        }
    }

    #[test]
    fn test_encode_decode_v0x04_private_broker() {
        let relay_id = "12345678-1234-1234-1234-123456789abc";
        let broker = "mqtts://my-private-broker.example.com:8883";
        let psk = fake_psk();

        let token = encode_join_token(relay_id, broker, &psk).unwrap();
        let decoded = decode_join_token(&token).unwrap();
        assert_eq!(decoded.relay_id, relay_id);
        assert_eq!(decoded.broker_url, broker);
        assert_eq!(decoded.psk, psk);
    }

    #[test]
    fn test_v0x04_public_token_compact() {
        // 1 + 16 + 32 + 1 = 50 bytes → ceil(50*4/3) = 67 base64url chars (no pad).
        let relay_id = "a1b2c3d4-e5f6-7890-abcd-ef1234567890";
        let broker = format!("mqtts://{}:{}", DEFAULT_BROKERS[0].0, DEFAULT_BROKERS[0].1);
        let token = encode_join_token(relay_id, &broker, &fake_psk()).unwrap();
        assert_eq!(token.len(), 67);
    }

    #[test]
    fn test_v0x04_truncated_psk_rejected() {
        let uuid_bytes = uuid_to_bytes("a1b2c3d4-e5f6-7890-abcd-ef1234567890").unwrap();
        // 1 version + 16 uuid + 10 bytes (less than full PSK)
        let mut buf = vec![0x04];
        buf.extend_from_slice(&uuid_bytes);
        buf.extend_from_slice(&[0x11u8; 10]);
        let token = URL_SAFE_NO_PAD.encode(&buf);
        assert!(decode_join_token(&token).is_none());
    }

    #[test]
    fn test_encode_decode_v0x04_bare_host_port_private_broker() {
        // Regression: earlier draft used `tail.starts_with("mqtt")` as the
        // public/private discriminator, which misparsed bare `host:port`
        // brokers (accepted by parse_broker_url) as a 1-byte index and then
        // returned None. The length-1 discriminator handles this correctly.
        let relay_id = "12345678-1234-1234-1234-123456789abc";
        let broker = "my-private-broker.example.com:8883";
        let psk = fake_psk();

        let token = encode_join_token(relay_id, broker, &psk).unwrap();
        let decoded = decode_join_token(&token).unwrap();
        assert_eq!(decoded.relay_id, relay_id);
        assert_eq!(decoded.broker_url, broker);
        assert_eq!(decoded.psk, psk);
    }

    #[test]
    fn test_v0x04_layout_is_stable() {
        // Decode produces the exact byte sequence we expect: version || uuid ||
        // psk || broker_index. Pinned so a non-Rust implementation can verify
        // the layout without us also pinning a base64 alphabet/order quirk.
        let relay_id = "a1b2c3d4-e5f6-7890-abcd-ef1234567890";
        let mut psk = [0u8; 32];
        for (i, b) in psk.iter_mut().enumerate() {
            *b = i as u8;
        }
        let broker = format!("mqtts://{}:{}", DEFAULT_BROKERS[0].0, DEFAULT_BROKERS[0].1);
        let token = encode_join_token(relay_id, &broker, &psk).unwrap();
        let raw = URL_SAFE_NO_PAD.decode(token.as_bytes()).unwrap();

        assert_eq!(raw[0], 0x04);
        assert_eq!(&raw[1..17], &uuid_to_bytes(relay_id).unwrap());
        assert_eq!(&raw[17..49], &psk);
        assert_eq!(raw[49], 0); // broker index 0 = first DEFAULT_BROKERS entry
        assert_eq!(raw.len(), 50);
        assert_eq!(token.len(), 67);
    }
}
