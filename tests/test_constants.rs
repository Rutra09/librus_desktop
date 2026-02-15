//! Tests to verify API constants are correctly defined.

use librus_front::api::constants::*;

#[test]
fn api_url_is_https() {
    assert!(LIBRUS_API_URL.starts_with("https://"));
}

#[test]
fn api_url_correct() {
    assert_eq!(LIBRUS_API_URL, "https://api.librus.pl/2.0");
}

#[test]
fn token_url_correct() {
    assert_eq!(LIBRUS_API_TOKEN_URL, "https://api.librus.pl/OAuth/Token");
}

#[test]
fn authorization_is_basic() {
    assert!(LIBRUS_API_AUTHORIZATION.starts_with("Basic "));
}

#[test]
fn authorization_base64_decodable() {
    let b64_part = LIBRUS_API_AUTHORIZATION.strip_prefix("Basic ").unwrap();
    // Should decode without error
    let decoded = base64_decode(b64_part);
    assert!(decoded.is_some());
    // Should contain client_id:secret format
    let decoded_str = decoded.unwrap();
    assert!(decoded_str.contains(':'), "Decoded auth should contain ':'");
}

fn base64_decode(input: &str) -> Option<String> {
    // Simple base64 decode check
    let bytes: Result<Vec<u8>, _> = base64_manual_decode(input);
    bytes.ok().and_then(|b| String::from_utf8(b).ok())
}

fn base64_manual_decode(input: &str) -> Result<Vec<u8>, ()> {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    let input = input.trim_end_matches('=');
    let mut output = Vec::new();
    let mut buf: u32 = 0;
    let mut bits: u32 = 0;

    for &byte in input.as_bytes() {
        let val = TABLE.iter().position(|&b| b == byte).ok_or(())? as u32;
        buf = (buf << 6) | val;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            output.push((buf >> bits) as u8);
            buf &= (1 << bits) - 1;
        }
    }

    Ok(output)
}

#[test]
fn user_agent_contains_librus() {
    assert!(LIBRUS_USER_AGENT.contains("Librus"));
}

#[test]
fn portal_url_correct() {
    assert_eq!(LIBRUS_PORTAL_URL, "https://portal.librus.pl/api");
}

#[test]
fn synergia_url_correct() {
    assert_eq!(LIBRUS_SYNERGIA_URL, "https://synergia.librus.pl");
}

#[test]
fn messages_url_correct() {
    assert_eq!(LIBRUS_MESSAGES_URL, "https://wiadomosci.librus.pl/module");
}

#[test]
fn client_id_not_empty() {
    assert!(!LIBRUS_CLIENT_ID.is_empty());
}

#[test]
fn redirect_url_is_app_scheme() {
    assert!(LIBRUS_REDIRECT_URL.starts_with("app://"));
}
