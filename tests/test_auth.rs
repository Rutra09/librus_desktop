//! Tests for auth state validation logic.

use librus_front::api::auth::AuthState;

fn make_auth(expires_at: i64) -> AuthState {
    AuthState {
        access_token: "test_token".into(),
        expires_at,
        portal_access_token: "portal_tok".into(),
        portal_refresh_token: "portal_ref".into(),
        portal_expires_at: chrono::Utc::now().timestamp() + 86400,
        synergia_login: "1234567u".into(),
        portal_email: "test@example.com".into(),
        portal_password: "secret".into(),
    }
}

#[test]
fn auth_state_valid_when_not_expired() {
    let now = chrono::Utc::now().timestamp();
    let auth = make_auth(now + 3600);
    assert!(auth.is_valid());
}

#[test]
fn auth_state_invalid_when_expired() {
    let now = chrono::Utc::now().timestamp();
    let auth = make_auth(now - 100);
    assert!(!auth.is_valid());
}

#[test]
fn auth_state_invalid_within_30s_margin() {
    let now = chrono::Utc::now().timestamp();
    let auth = make_auth(now + 20); // only 20s left < 30s margin
    assert!(!auth.is_valid());
}

#[test]
fn auth_state_valid_at_exactly_31s() {
    let now = chrono::Utc::now().timestamp();
    let auth = make_auth(now + 31); // 31s left > 30s margin
    assert!(auth.is_valid());
}

#[test]
fn auth_state_clone() {
    let auth = make_auth(9999999999);
    let cloned = auth.clone();
    assert_eq!(cloned.access_token, "test_token");
    assert_eq!(cloned.portal_email, "test@example.com");
    assert_eq!(cloned.synergia_login, "1234567u");
    assert_eq!(cloned.expires_at, 9999999999);
}

#[test]
fn portal_token_validity() {
    let now = chrono::Utc::now().timestamp();
    let mut auth = make_auth(now + 3600);

    auth.portal_expires_at = now + 3600;
    assert!(auth.is_portal_valid());

    auth.portal_expires_at = now + 10; // within 30s margin
    assert!(!auth.is_portal_valid());

    auth.portal_expires_at = now - 100;
    assert!(!auth.is_portal_valid());
}
