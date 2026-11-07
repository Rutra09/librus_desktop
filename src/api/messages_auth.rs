use anyhow::Context;
use reqwest::cookie::CookieStore;

use super::constants::*;
use super::http;

/// Why the messages handshake failed.
///
/// The caller needs this distinction to decide whether refreshing the Synergia
/// token is worth a round trip. Collapsing every failure into a `String` (the
/// old behaviour) meant a 503 from `api.librus.pl` looked exactly like an
/// expired bearer token — and the retry path fired the wrong recovery.
#[derive(Debug, thiserror::Error)]
pub enum MessagesAuthError {
    /// `api.librus.pl` refused our bearer token: expired, revoked, or the
    /// account is not allowed to open a Synergia session.
    #[error("api.librus.pl odrzuciło token (HTTP {status}): {body}")]
    TokenRejected { status: u16, body: String },

    /// Synergia answered, but no DZIENNIKSID cookie was issued.
    #[error("Synergia nie wystawiła ciasteczka DZIENNIKSID (sesja nie powstała)")]
    NoSession,

    /// Synergia is in `przerwa_techniczna` — retrying is pointless right now.
    #[error("Synergia jest w przerwie technicznej")]
    Maintenance,

    /// Synergia wants a reCAPTCHA (`MultiDomainLogon`); we cannot solve it.
    #[error("Logowanie do wiadomości wymaga captchy (MultiDomainLogon)")]
    Captcha,

    /// Anything else — network, parsing, unexpected redirect.
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl MessagesAuthError {
    /// Whether a fresh Synergia token could plausibly fix this.
    pub fn is_token_related(&self) -> bool {
        matches!(self, Self::TokenRejected { .. } | Self::NoSession)
    }
}

/// Authenticate with the Messages module to get DZIENNIKSID cookie.
///
/// Uses the API AutoLoginToken endpoint to get a Synergia token, then uses that
/// token to access Synergia and get the messages session cookie.
/// Returns (messages_cookie, synergia_cookie, expiry)
pub async fn login_messages(
    auth: &super::auth::AuthState,
) -> Result<(String, String, i64), MessagesAuthError> {
    // Step 1: Get AutoLoginToken from API
    let client = http::client();

    let token_response = client
        .post(&format!("{}/AutoLoginToken", LIBRUS_API_URL))
        .header("Authorization", format!("Bearer {}", &auth.access_token))
        .send()
        .await
        .context("Failed to request AutoLoginToken")?;

    let token_status = token_response.status();
    let token_body = token_response
        .text()
        .await
        .context("Failed to read AutoLoginToken response")?;

    // The status has to be checked *before* parsing: on an expired token the
    // API answers 401 with `{"Code":"TokenIsExpired"}` and no `Token` field at
    // all, and reporting that as "AutoLoginToken not found in response" hid the
    // one piece of information worth having.
    if !token_status.is_success() {
        return Err(MessagesAuthError::TokenRejected {
            status: token_status.as_u16(),
            body: token_body.chars().take(300).collect(),
        });
    }

    let token_json: serde_json::Value = serde_json::from_str(&token_body)
        .with_context(|| format!("AutoLoginToken: nieprawidłowy JSON: {token_body}"))?;

    let auto_login_token = token_json["Token"]
        .as_str()
        .ok_or_else(|| MessagesAuthError::TokenRejected {
            status: token_status.as_u16(),
            body: token_body.chars().take(300).collect(),
        })?;

    // Step 2: Use token to establish Synergia session and get cookies
    let jar = std::sync::Arc::new(reqwest::cookie::Jar::default());
    let client_with_cookies = http::builder()
        .user_agent(LIBRUS_USER_AGENT)
        .cookie_provider(std::sync::Arc::clone(&jar))
        .redirect(reqwest::redirect::Policy::none()) // Manual redirect handling
        .build()
        .context("Failed to create HTTP client with cookies")?;

    // Step 2a: Establish Synergia session.
    // Follow redirects manually until we hit a final page (likely uczen/index).
    let mut current_url = format!(
        "https://synergia.librus.pl/loguj/token/{}/przenies",
        auto_login_token
    );

    for _ in 0..10 {
        let response = client_with_cookies
            .get(&current_url)
            .send()
            .await
            .context("Failed to send request in Synergia login loop")?;

        let status = response.status();

        if status.is_redirection() {
            if let Some(location) = response.headers().get("location").and_then(|v| v.to_str().ok()) {
                if location.contains("przerwa_techniczna") {
                    return Err(MessagesAuthError::Maintenance);
                }
                current_url = resolve(current_url.as_str(), location)?;
                continue;
            }
        }

        if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
            return Err(MessagesAuthError::TokenRejected {
                status: status.as_u16(),
                body: "Synergia odrzuciła token logowania".to_string(),
            });
        }

        break;
    }

    // Step 2b: Access the messages module to trigger AutoLogon, which is what
    // sets DZIENNIKSID for wiadomosci.librus.pl.
    current_url = "https://synergia.librus.pl/wiadomosci2".to_string();

    for _ in 0..10 {
        let response = client_with_cookies
            .get(&current_url)
            .send()
            .await
            .context("Failed to send request in Messages login loop")?;

        let status = response.status();

        if status.is_redirection() {
            if let Some(location) = response.headers().get("location").and_then(|v| v.to_str().ok()) {
                if location.contains("AutoLogon") {
                    // Cookies are already in the jar; following this redirect
                    // would only burn the one-time session id.
                    break;
                }
                if location.contains("MultiDomainLogon") {
                    return Err(MessagesAuthError::Captcha);
                }
                current_url = resolve(current_url.as_str(), location)?;
                continue;
            }
        }

        break;
    }

    // Extract cookies from the jar — they are not in the response headers.
    let url_wiadomosci = "https://wiadomosci.librus.pl"
        .parse::<reqwest::Url>()
        .context("Failed to parse wiadomosci URL")?;

    let cookies_str = jar
        .cookies(&url_wiadomosci)
        .and_then(|h| h.to_str().ok().map(str::to_string))
        .unwrap_or_default();

    let url_synergia = "https://synergia.librus.pl"
        .parse::<reqwest::Url>()
        .context("Failed to parse synergia URL")?;
    let synergia_cookies_str = jar
        .cookies(&url_synergia)
        .and_then(|h| h.to_str().ok().map(str::to_string))
        .unwrap_or_default();

    // Parse DZIENNIKSID and sanitize it (-MAINT suffix shows up during
    // maintenance windows, see the Android reference implementation).
    let dzienniksid = extract_cookie(&cookies_str, "DZIENNIKSID")
        .or_else(|| extract_cookie(&synergia_cookies_str, "DZIENNIKSID"))
        .map(|v| v.replace("-MAINT", "").replace("MAINT", ""));

    match dzienniksid {
        Some(sid) if !sid.is_empty() => {
            let expiry = chrono::Utc::now().timestamp() + (45 * 60);
            // Only DZIENNIKSID: extra cookies from the redirect chain have been
            // observed to make the XML API answer `eAccessDeny`.
            Ok((format!("DZIENNIKSID={}", sid), synergia_cookies_str, expiry))
        }
        _ => Err(MessagesAuthError::NoSession),
    }
}

/// Resolve a `Location` header against the URL that produced it.
fn resolve(base: &str, location: &str) -> anyhow::Result<String> {
    if location.starts_with("http") {
        return Ok(location.to_string());
    }
    let base_url = reqwest::Url::parse(base)?;
    Ok(base_url.join(location)?.to_string())
}

/// Pull one cookie's value out of a `Cookie:` header string.
fn extract_cookie(header: &str, name: &str) -> Option<String> {
    header.split(';').find_map(|pair| {
        let pair = pair.trim();
        pair.strip_prefix(name)
            .and_then(|rest| rest.strip_prefix('='))
            .map(str::to_string)
    })
}
