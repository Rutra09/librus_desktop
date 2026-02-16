use anyhow::{Context, Result, bail};
use reqwest::Client;
use reqwest::redirect::Policy;

use super::constants::*;

/// Holds the current authentication state (tokens + expiry).
#[derive(Debug, Clone)]
pub struct AuthState {
    pub access_token: String,
    /// Token expiry as a UNIX timestamp (seconds).
    pub expires_at: i64,
    pub refresh_token: String,
    /// Portal access token (for refreshing Synergia token).
    pub portal_access_token: String,
    pub portal_refresh_token: String,
    pub portal_expires_at: i64,
    pub user_id: i64,
    pub portal_email: String,
    pub portal_password: String,
    pub synergia_login: String,
    
    // Synergia credentials for messages module
    pub synergia_username: Option<String>,
    pub synergia_password: Option<String>,
    
    // Messages session
    pub messages_session_id: Option<String>,
    pub messages_session_expiry: Option<i64>,
    
    // Synergia session (for scraping)
    pub synergia_cookie: Option<String>,
}

impl AuthState {
    /// Check if the Synergia API access token is still valid (with 30s margin).
    pub fn is_valid(&self) -> bool {
        let now = chrono::Utc::now().timestamp();
        self.expires_at - 30 > now
    }

    /// Check if the portal token is still valid.
    pub fn is_portal_valid(&self) -> bool {
        let now = chrono::Utc::now().timestamp();
        self.portal_expires_at - 30 > now
    }
}

/// Execute the full Portal OAuth2 login flow and extract a Synergia API token.
///
/// Flow:
/// 1. GET authorize URL → HTML login page
/// 2. Extract CSRF token and hidden form fields
/// 3. POST email + password to login action
/// 4. Follow redirects until we get `app://librus?code=XXX`
/// 5. Exchange code for portal access_token + refresh_token
/// 6. GET `/api/v3/SynergiaAccounts/fresh/{login}` → Synergia API token
pub async fn login_portal(
    email: &str,
    password: &str,
) -> Result<AuthState> {
    // Use a client that does NOT auto-follow redirects (we need to intercept them)
    let client = Client::builder()
        .user_agent(LIBRUS_USER_AGENT)
        .redirect(Policy::none())
        .cookie_store(true)
        .build()
        .context("Nie udało się utworzyć klienta HTTP")?;

    // Step 1: GET authorize URL → get login page or redirect chain
    let auth_code = do_authorize_and_login(&client, email, password).await?;

    // Step 2: Exchange authorization code for portal tokens
    let (portal_access_token, portal_refresh_token, portal_expires_in) =
        exchange_code(&auth_code).await?;

    let now = chrono::Utc::now().timestamp();
    let portal_expires_at = now + portal_expires_in;

    // Step 3: Get list of Synergia accounts and extract the API token
    let (synergia_login, api_access_token) =
        extract_synergia_token(&portal_access_token).await?;

    Ok(AuthState {
        access_token: api_access_token,
        expires_at: now + 6 * 60 * 60, // 6 hours (same as szkolny-android)
        refresh_token: portal_refresh_token.clone(),
        portal_access_token,
        portal_refresh_token,
        portal_expires_at,
        user_id: 0,  // Not used currently
        synergia_login,
        portal_email: email.to_string(),
        portal_password: password.to_string(),
        synergia_username: None,  // Will be set later by user
        synergia_password: None,  // Will be set later by user
        messages_session_id: None,
        messages_session_expiry: None,
        synergia_cookie: None,
    })
}

/// Steps 1-3: Authorize → login → get auth code via redirects
async fn do_authorize_and_login(
    client: &Client,
    email: &str,
    password: &str,
) -> Result<String> {
    // Step 1: GET the authorize URL
    let mut url = LIBRUS_AUTHORIZE_URL.to_string();
    let mut csrf_token: Option<String> = None;
    let hidden_params: Vec<(String, String)>;
    let mut login_url = LIBRUS_LOGIN_URL.to_string();

    // Follow redirects manually until we get the login page
    loop {
        let response = client
            .get(&url)
            .header("X-Requested-With", LIBRUS_HEADER)
            .send()
            .await
            .context("Nie udało się połączyć z portalem Librus")?;

        let status = response.status();

        if let Some(location) = response.headers().get("location").and_then(|v| v.to_str().ok()) {
            let location = location.to_string();

            // Check if we got the auth code in a redirect
            if let Some(code) = extract_code_from_url(&location) {
                return Ok(code);
            }

            // Follow the redirect
            url = if location.starts_with('/') {
                format!("https://portal.librus.pl{}", location)
            } else {
                location
            };
            continue;
        }

        if !status.is_success() && !status.is_redirection() {
            bail!("Portal Librus zwrócił błąd HTTP {}", status.as_u16());
        }

        // We got the login page — parse CSRF token and hidden fields
        let body = response.text().await?;

        // Check for errors in the response
        if body.contains("Sesja logowania wygasła") {
            bail!("Sesja logowania wygasła — spróbuj ponownie");
        }

        // Extract CSRF token
        if let Some(token) = extract_csrf_token(&body) {
            csrf_token = Some(token);
        }

        // Extract login form action URL
        if let Some(action_url) = extract_login_form_action(&body) {
            login_url = action_url;
        }

        // Extract hidden input fields
        hidden_params = extract_hidden_inputs(&body);

        break;
    }

    // Step 2: POST login credentials
    let mut form_params = hidden_params;
    form_params.push(("email".to_string(), email.to_string()));
    form_params.push(("password".to_string(), password.to_string()));

    let mut login_response = client
        .post(&login_url)
        .header("X-Requested-With", LIBRUS_HEADER)
        .header("Referer", &url);

    if let Some(ref token) = csrf_token {
        login_response = login_response.header("X-CSRF-TOKEN", token);
    }

    let login_response = login_response
        .form(&form_params)
        .send()
        .await
        .context("Nie udało się wysłać formularza logowania")?;

    // Step 3: Follow redirects after login until we find the auth code
    let mut redirect_url = if let Some(location) = login_response.headers().get("location").and_then(|v| v.to_str().ok()) {
        location.to_string()
    } else {
        let body = login_response.text().await?;
        if body.contains("Upewnij się, że nie") || body.contains("Podany adres e-mail jest nieprawidłowy") {
            bail!("Nieprawidłowy email lub hasło");
        }
        if body.contains("robotem") || body.contains("g-recaptcha") || body.contains("captchaValidate") {
            bail!("Wymagana captcha — spróbuj ponownie później");
        }
        // Maybe the login succeeded and redirected back to authorize
        LIBRUS_AUTHORIZE_URL.to_string()
    };

    // Follow redirect chain to find the code
    for _ in 0..10 {
        if let Some(code) = extract_code_from_url(&redirect_url) {
            return Ok(code);
        }

        if redirect_url == format!("{}?command=close", LIBRUS_REDIRECT_URL) {
            bail!("Portal Librus jest w trybie konserwacji");
        }

        let full_url = if redirect_url.starts_with('/') {
            format!("https://portal.librus.pl{}", redirect_url)
        } else {
            redirect_url.clone()
        };

        let response = client
            .get(&full_url)
            .header("X-Requested-With", LIBRUS_HEADER)
            .send()
            .await?;

        redirect_url = match response.headers().get("location").and_then(|v| v.to_str().ok()) {
            Some(loc) => loc.to_string(),
            None => bail!("Nie udało się uzyskać kodu autoryzacji — brak przekierowania"),
        };
    }

    // Last chance: check the final redirect URL
    extract_code_from_url(&redirect_url)
        .ok_or_else(|| anyhow::anyhow!("Nie udało się uzyskać kodu autoryzacji po logowaniu"))
}

/// Exchange an authorization code for portal tokens.
async fn exchange_code(
    code: &str,
) -> Result<(String, String, i64)> {
    let params = [
        ("client_id", LIBRUS_CLIENT_ID),
        ("grant_type", "authorization_code"),
        ("code", code),
        ("redirect_uri", LIBRUS_REDIRECT_URL),
    ];

    // Use a new client that follows redirects for this request
    let token_client = Client::builder()
        .user_agent(LIBRUS_USER_AGENT)
        .build()?;

    let response = token_client
        .post(LIBRUS_TOKEN_URL)
        .form(&params)
        .send()
        .await
        .context("Nie udało się wymienić kodu na token")?;

    let status = response.status();
    let body = response.text().await?;

    let json: serde_json::Value =
        serde_json::from_str(&body).context("Nieprawidłowa odpowiedź token endpoint")?;

    // Check for errors
    if let Some(error) = json.get("error").or(json.get("hint")).and_then(|v| v.as_str()) {
        bail!("Błąd wymiany kodu: {error}");
    }

    if !status.is_success() {
        bail!("Token endpoint HTTP {}: {body}", status.as_u16());
    }

    let access_token = json["access_token"]
        .as_str()
        .context("Brak access_token w odpowiedzi portalu")?
        .to_string();

    let refresh_token = json["refresh_token"]
        .as_str()
        .context("Brak refresh_token w odpowiedzi portalu")?
        .to_string();

    let expires_in = json["expires_in"].as_i64().unwrap_or(86400);

    Ok((access_token, refresh_token, expires_in))
}

/// Refresh portal tokens using the refresh token.
pub async fn refresh_portal_token(
    portal_refresh_token: &str,
) -> Result<(String, String, i64)> {
    let params = [
        ("client_id", LIBRUS_CLIENT_ID),
        ("grant_type", "refresh_token"),
        ("refresh_token", portal_refresh_token),
    ];

    let client = Client::builder()
        .user_agent(LIBRUS_USER_AGENT)
        .build()?;

    let response = client
        .post(LIBRUS_TOKEN_URL)
        .form(&params)
        .send()
        .await
        .context("Nie udało się odświeżyć tokena portalowego")?;

    let body = response.text().await?;
    let json: serde_json::Value = serde_json::from_str(&body)?;

    if let Some(error) = json.get("error").or(json.get("hint")).and_then(|v| v.as_str()) {
        bail!("Odświeżanie tokena portalu: {error}");
    }

    let access_token = json["access_token"]
        .as_str()
        .context("Brak access_token")?
        .to_string();
    let refresh_token = json["refresh_token"]
        .as_str()
        .context("Brak refresh_token")?
        .to_string();
    let expires_in = json["expires_in"].as_i64().unwrap_or(86400);

    Ok((access_token, refresh_token, expires_in))
}

/// Get Synergia API token from the portal using `SynergiaAccounts`.
///
/// The response format is: `{"accounts": [{"login": "...", "accessToken": "...", ...}]}`
///
/// Returns `(synergia_login, api_access_token)`.
async fn extract_synergia_token(
    portal_access_token: &str,
) -> Result<(String, String)> {
    let accounts_url = format!("{}{}", LIBRUS_PORTAL_URL, LIBRUS_ACCOUNTS_URL);

    let client = Client::builder()
        .user_agent(LIBRUS_USER_AGENT)
        .build()?;

    let response = client
        .get(&accounts_url)
        .header("Authorization", format!("Bearer {}", portal_access_token))
        .send()
        .await
        .context("Nie udało się pobrać listy kont Synergia")?;

    let body = response.text().await?;
    let json: serde_json::Value = serde_json::from_str(&body)?;

    // Check for errors
    if let Some(reason) = json.get("reason").or(json.get("message")).and_then(|v| v.as_str()) {
        bail!("Błąd pobierania kont: {reason}");
    }

    // Response is {"accounts": [...]} — extract the accounts array
    let accounts = json
        .get("accounts")
        .and_then(|v| v.as_array())
        .context("Brak tablicy 'accounts' w odpowiedzi SynergiaAccounts")?;

    if accounts.is_empty() {
        bail!("Brak kont Synergia powiązanych z tym kontem portal");
    }

    // Take the first account — it already contains the accessToken
    let account = &accounts[0];
    let login = account["login"]
        .as_str()
        .context("Brak loginu w koncie Synergia")?
        .to_string();

    let access_token = account["accessToken"]
        .as_str()
        .context("Brak accessToken w koncie Synergia")?
        .to_string();

    Ok((login, access_token))
}


/// Refresh the Synergia API token using portal credentials.
pub async fn refresh_synergia_token(auth: &AuthState) -> Result<AuthState> {
    let now = chrono::Utc::now().timestamp();
    let mut new_auth = auth.clone();

    // Refresh portal token if needed
    if !auth.is_portal_valid() {
        let (portal_access, portal_refresh, portal_expires_in) =
            refresh_portal_token(&auth.portal_refresh_token).await?;
        new_auth.portal_access_token = portal_access;
        new_auth.portal_refresh_token = portal_refresh;
        new_auth.portal_expires_at = now + portal_expires_in;
    }

    // Use a client for portal API
    let client = Client::builder()
        .user_agent(LIBRUS_USER_AGENT)
        .build()?;

    // Get fresh Synergia token from SynergiaAccounts
    let accounts_url = format!("{}{}", LIBRUS_PORTAL_URL, LIBRUS_ACCOUNTS_URL);

    let response = client
        .get(&accounts_url)
        .header(
            "Authorization",
            format!("Bearer {}", new_auth.portal_access_token),
        )
        .send()
        .await?;

    let body = response.text().await?;
    let json: serde_json::Value = serde_json::from_str(&body)?;

    if let Some(reason) = json.get("reason").or(json.get("message")).and_then(|v| v.as_str()) {
        bail!("Odświeżanie tokena Synergia: {reason}");
    }

    let accounts = json
        .get("accounts")
        .and_then(|v| v.as_array())
        .context("Brak tablicy 'accounts' w odpowiedzi")?;

    // Find the account matching our synergia_login
    let account = accounts
        .iter()
        .find(|a| a["login"].as_str() == Some(&auth.synergia_login))
        .or_else(|| accounts.first())
        .context("Brak konta Synergia")?;

    let access_token = account["accessToken"]
        .as_str()
        .context("Brak accessToken")?
        .to_string();

    new_auth.access_token = access_token;
    new_auth.expires_at = now + 6 * 60 * 60;

    Ok(new_auth)
}

// ---- Helper functions for HTML parsing ----

fn extract_code_from_url(url: &str) -> Option<String> {
    if url.starts_with(LIBRUS_REDIRECT_URL) {
        if let Some(query) = url.split('?').nth(1) {
            for param in query.split('&') {
                if let Some(value) = param.strip_prefix("code=") {
                    return Some(value.to_string());
                }
            }
        }
    }
    None
}

fn extract_csrf_token(html: &str) -> Option<String> {
    // Look for <meta name="csrf-token" content="...">
    let patterns = [
        r#"name="csrf-token" content=""#,
        r#"content="" name="csrf-token""#,
        r#"name="_token" value=""#,
    ];

    for pattern in patterns {
        if let Some(start) = html.find(pattern) {
            let after = &html[start + pattern.len()..];
            if let Some(end) = after.find('"') {
                return Some(after[..end].to_string());
            }
        }
    }

    // Also try <input type="hidden" name="_token" value="...">
    if let Some(start) = html.find(r#"name="_token""#) {
        let region = &html[start.saturating_sub(100)..html.len().min(start + 200)];
        if let Some(val_start) = region.find(r#"value=""#) {
            let after = &region[val_start + 7..];
            if let Some(end) = after.find('"') {
                return Some(after[..end].to_string());
            }
        }
    }

    None
}

fn extract_login_form_action(html: &str) -> Option<String> {
    // Find <form> tags with "login" and "post" in them
    let lower = html.to_lowercase();
    let mut pos = 0;
    while let Some(start) = lower[pos..].find("<form") {
        let abs_start = pos + start;
        if let Some(end) = lower[abs_start..].find('>') {
            let form_tag = &lower[abs_start..abs_start + end + 1];
            if form_tag.contains("login") && form_tag.contains("post") {
                // Extract action attribute from the original (case-preserved) HTML
                let orig_tag = &html[abs_start..abs_start + end + 1];
                if let Some(action_start) = orig_tag.find("action=\"") {
                    let after = &orig_tag[action_start + 8..];
                    if let Some(action_end) = after.find('"') {
                        return Some(after[..action_end].to_string());
                    }
                }
            }
            pos = abs_start + end + 1;
        } else {
            break;
        }
    }
    None
}

fn extract_hidden_inputs(html: &str) -> Vec<(String, String)> {
    let mut result = Vec::new();
    let lower = html.to_lowercase();
    let mut pos = 0;

    while let Some(start) = lower[pos..].find("<input") {
        let abs_start = pos + start;
        if let Some(end) = lower[abs_start..].find('>') {
            let tag = &html[abs_start..abs_start + end + 1];
            let tag_lower = tag.to_lowercase();

            if tag_lower.contains("type=\"hidden\"") {
                let name = extract_attr(tag, "name");
                let value = extract_attr(tag, "value");
                if let (Some(n), Some(v)) = (name, value) {
                    result.push((n, v));
                }
            }
            pos = abs_start + end + 1;
        } else {
            break;
        }
    }

    result
}

fn extract_attr(tag: &str, attr_name: &str) -> Option<String> {
    let pattern = format!("{}=\"", attr_name);
    // case-insensitive search
    let lower = tag.to_lowercase();
    if let Some(start) = lower.find(&pattern) {
        let after = &tag[start + pattern.len()..];
        if let Some(end) = after.find('"') {
            return Some(after[..end].to_string());
        }
    }
    None
}
