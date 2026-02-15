use anyhow::{Context, Result, bail};
use reqwest::Client;
use serde_json::Value;
use tokio::sync::Mutex;
use std::sync::Arc;

use super::auth::{self, AuthState};
use super::constants::*;

/// High-level Librus API client that manages authentication and requests.
#[derive(Clone)]
pub struct LibrusClient {
    http: Client,
    auth: Arc<Mutex<AuthState>>,
}

impl LibrusClient {
    /// Create a new client from an existing auth state.
    pub fn new(auth: AuthState) -> Self {
        let http = Client::builder()
            .user_agent(LIBRUS_USER_AGENT)
            .build()
            .expect("Failed to create HTTP client");

        Self {
            http,
            auth: Arc::new(Mutex::new(auth)),
        }
    }

    /// Login with Portal email + password and return a new client.
    ///
    /// Uses the Portal OAuth2 flow to obtain a Synergia API token.
    pub async fn login(email: &str, password: &str) -> Result<Self> {
        let auth = auth::login_portal(email, password).await?;
        let http = Client::builder()
            .user_agent(LIBRUS_USER_AGENT)
            .build()
            .expect("Failed to create HTTP client");

        Ok(Self {
            http,
            auth: Arc::new(Mutex::new(auth)),
        })
    }

    /// Ensure the access token is valid; refresh if needed.
    async fn ensure_token(&self) -> Result<String> {
        let mut auth = self.auth.lock().await;

        if !auth.is_valid() {
            // Try refreshing the Synergia token via portal
            match auth::refresh_synergia_token(&auth).await {
                Ok(new_auth) => {
                    *auth = new_auth;
                }
                Err(_) => {
                    // Refresh failed — full re-login
                    let email = auth.portal_email.clone();
                    let password = auth.portal_password.clone();
                    let new_auth = auth::login_portal(&email, &password).await?;
                    *auth = new_auth;
                }
            }
        }

        Ok(auth.access_token.clone())
    }

    /// Perform a GET request to a Librus API endpoint.
    ///
    /// `endpoint` is relative to `api.librus.pl/2.0/`, e.g. `"Grades"` or `"Timetables?weekStart=2024-01-01"`.
    pub async fn api_get(&self, endpoint: &str) -> Result<Value> {
        let token = self.ensure_token().await?;
        let url = format!("{}/{}", LIBRUS_API_URL, endpoint);

        let response = self
            .http
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .with_context(|| format!("Nie udało się połączyć z {}", url))?;

        let status = response.status();

        // Handle 503 — maintenance
        if status.as_u16() == 503 {
            bail!("Librus API jest w trybie konserwacji (503)");
        }

        let body = response
            .text()
            .await
            .context("Nie udało się odczytać odpowiedzi")?;

        if body.is_empty() {
            bail!("Pusta odpowiedź z serwera dla endpoint: {endpoint}");
        }

        let json: Value =
            serde_json::from_str(&body).with_context(|| {
                format!("Nieprawidłowy JSON z endpoint {endpoint}: {body}")
            })?;

        // Check for API-level errors
        if !status.is_success() {
            let code = json
                .get("Code")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown");
            let message = json
                .get("Message")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            match code {
                "TokenIsExpired" => {
                    // Token expired mid-request — retry once with a fresh token
                    drop(json);
                    let mut auth = self.auth.lock().await;
                    let new_auth = auth::refresh_synergia_token(&auth).await?;
                    *auth = new_auth;
                    let new_token = auth.access_token.clone();
                    drop(auth);

                    let retry_response = self
                        .http
                        .get(&url)
                        .header("Authorization", format!("Bearer {}", new_token))
                        .send()
                        .await?;

                    let retry_body = retry_response.text().await?;
                    let retry_json: Value = serde_json::from_str(&retry_body)?;
                    return Ok(retry_json);
                }
                "Insufficient scopes" => {
                    bail!("Brak uprawnień do zasobu: {endpoint}");
                }
                "AccessDeny" | "Request is denied" => {
                    bail!("Odmowa dostępu: {message}");
                }
                "Resource not found" | "NotFound" => {
                    bail!("Zasób nie znaleziony: {endpoint}");
                }
                "LuckyNumberIsNotActive" => {
                    bail!("Szczęśliwy numer nie jest aktywny w tej szkole");
                }
                "NotesIsNotActive" => {
                    bail!("Uwagi nie są aktywne w tej szkole");
                }
                _ => {
                    bail!("Błąd API Librus [{code}]: {message}");
                }
            }
        }

        Ok(json)
    }

    /// Perform a POST request to a Librus API endpoint with a JSON body.
    pub async fn api_post(&self, endpoint: &str, body: &Value) -> Result<Value> {
        let token = self.ensure_token().await?;
        let url = format!("{}/{}", LIBRUS_API_URL, endpoint);

        let response = self
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .json(body)
            .send()
            .await
            .with_context(|| format!("Nie udało się połączyć z {}", url))?;

        let status = response.status();
        let resp_body = response.text().await?;

        if resp_body.is_empty() {
            bail!("Pusta odpowiedź z serwera dla endpoint: {endpoint}");
        }

        let json: Value = serde_json::from_str(&resp_body)?;

        if !status.is_success() {
            let code = json
                .get("Code")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown");
            let message = json
                .get("Message")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            bail!("Błąd API [{code}]: {message}");
        }

        Ok(json)
    }

    /// Get a copy of the current auth state (for persistence).
    pub async fn get_auth_state(&self) -> AuthState {
        self.auth.lock().await.clone()
    }

    /// Get messages session if valid, otherwise None.
    pub async fn get_messages_session(&self) -> Option<(String, i64)> {
        let auth = self.auth.lock().await;
        if let (Some(sid), Some(exp)) = (&auth.messages_session_id, auth.messages_session_expiry) {
            Some((sid.clone(), exp))
        } else {
            None
        }
    }

    /// Update messages session credentials.
    pub async fn set_messages_session(&self, session_id: String, expiry: i64) {
        let mut auth = self.auth.lock().await;
        auth.messages_session_id = Some(session_id);
        auth.messages_session_expiry = Some(expiry);
    }
}
