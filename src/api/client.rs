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

    /// Fetch homework entries.
    /// 
    /// `date_from` and `date_to` are optional. If not provided, defaults to current week or similar by API.
    pub async fn fetch_homework(&self, _date_from: Option<&str>, _date_to: Option<&str>) -> Result<Vec<crate::api::models::Event>> {
        let endpoint = "HomeWorks".to_string();
        
        // if let (Some(from), Some(to)) = (date_from, date_to) {
        //     endpoint = format!("HomeWorks?dateFrom={}&dateTo={}", from, to);
        // } else if let Some(from) = date_from {
        //     endpoint = format!("HomeWorks?dateFrom={}", from);
        // }
        // endpoint = "HomeWorks".to_string();

        let json = self.api_get(&endpoint).await?;
        let response: crate::api::models::EventsResponse = serde_json::from_value(json)?;
        Ok(response.homeworks.unwrap_or_default())
    }

    /// Fetch homework categories.
    pub async fn fetch_homework_categories(&self) -> Result<Vec<crate::api::models::HomeworkCategory>> {
        let json = self.api_get("HomeWorks/Categories").await?;
        let response: crate::api::models::HomeworkCategoryResponse = serde_json::from_value(json)?;
        Ok(response.categories.unwrap_or_default())
    }

    /// Update messages session credentials.
    pub async fn set_messages_session(&self, session_id: String, synergia_cookie: String, expiry: i64) {
        let mut auth = self.auth.lock().await;
        auth.messages_session_id = Some(session_id);
        auth.synergia_cookie = Some(synergia_cookie);
        auth.messages_session_expiry = Some(expiry);
    }
    /// Fetch homework entries via Synergia scraping.
    pub async fn fetch_homework_via_synergia(&self, date_from: Option<&str>, date_to: Option<&str>) -> Result<Vec<crate::api::models::Event>> {
        let url = "https://synergia.librus.pl/moje_zadania";
        
        // Ensure we have a valid Synergia session
        let synergia_cookie = self.get_synergia_session().await?;
        
        // Parse dates or use defaults
        let start_date = if let Some(from) = date_from {
            chrono::NaiveDate::parse_from_str(from, "%Y-%m-%d").unwrap_or_else(|_| chrono::Local::now().date_naive())
        } else {
            chrono::Local::now().date_naive()
        };

        let end_date = if let Some(to) = date_to {
            chrono::NaiveDate::parse_from_str(to, "%Y-%m-%d").unwrap_or_else(|_| chrono::Local::now().date_naive() + chrono::Duration::days(7))
        } else {
            chrono::Local::now().date_naive() + chrono::Duration::days(7)
        };

        let mut events = Vec::new();
        let mut current_start = start_date;
        let mut id_counter = 100000;

        // Loop in 28-day chunks (safe margin within 1 month)
        while current_start < end_date {
            let mut current_end = current_start + chrono::Duration::days(28);
            if current_end > end_date {
                current_end = end_date;
            }

            let from_str = current_start.format("%Y-%m-%d").to_string();
            let to_str = current_end.format("%Y-%m-%d").to_string();
            
            let mut params = std::collections::HashMap::new();
            params.insert("dataOd", from_str.as_str());
            params.insert("dataDo", to_str.as_str());
            params.insert("przedmiot", "-1");

            // Create a new client that sends the Synergia cookie
            let client = reqwest::Client::builder()
                .user_agent(crate::api::constants::LIBRUS_USER_AGENT)
                .redirect(reqwest::redirect::Policy::none()) 
                .build()?;
            
            let response = client.post(url)
                .header("Cookie", &synergia_cookie)
                .header("Referer", "https://synergia.librus.pl/rodzic/index")
                .header("Origin", "https://synergia.librus.pl")
                .form(&params)
                .send()
                .await?;
                
            if !response.status().is_success() {
                 if response.status().is_redirection() {
                     bail!("Synergia redirected (session expired?)");
                 }
            } else {
                let html = response.text().await?;
                let document = scraper::Html::parse_document(&html);
                let table_selector = scraper::Selector::parse("table.myHomeworkTable > tbody > tr").unwrap();
                let td_selector = scraper::Selector::parse("td").unwrap();
                
                for row in document.select(&table_selector) {
                    let cells: Vec<_> = row.select(&td_selector).collect();
                    if cells.len() >= 10 {
                        let subject_name = cells[0].text().collect::<String>().trim().to_string();
                        let teacher_name = cells[1].text().collect::<String>().trim().to_string();
                        let topic = cells[2].text().collect::<String>().trim().to_string();
                        // cells[3] is options
                        // cells[4] is date added?
                        // cells[5] is options?
                        // cells[6] is date of event?
                        let event_date_str = cells[6].text().collect::<String>().trim().to_string();
                        
                        // Extract ID
                        let input_selector = scraper::Selector::parse("input").unwrap();
                        let mut homework_id = id_counter;
                        if let Some(input) = cells[9].select(&input_selector).next() {
                            if let Some(onclick) = input.value().attr("onclick") {
                                if let Some(start) = onclick.find("/podglad/") {
                                     let end_part = &onclick[start + 9..];
                                     if let Some(end) = end_part.find('\'') {
                                         if let Ok(id) = end_part[0..end].parse::<i64>() {
                                             homework_id = id;
                                         }
                                     }
                                }
                            }
                        }
                        id_counter += 1;

                        events.push(crate::api::models::Event {
                            id: Some(homework_id),
                            date: Some(event_date_str),
                            time_from: None,
                            lesson_no: None,
                            content: Some(topic),
                            category: Some(crate::api::models::IdRef { id: Some(-1) }), // -1 for scraped homework
                            subject: Some(crate::api::models::IdRef { id: Some(0) }), // 0 for scraped subject
                            created_by: Some(crate::api::models::IdRef { id: Some(0) }),
                            class: None,
                            add_date: None,
                            scraped_subject: Some(subject_name),
                            scraped_teacher: Some(teacher_name),
                            scraped_category: Some("Zadanie domowe".to_string()),
                        });
                    }
                }
            }
            
            // Move to next chunk (inclusive logic: +1 day)
            current_start = current_end + chrono::Duration::days(1);
        }
        
        Ok(events)

    }

    /// Attempt to login to messages, refreshing tokens if needed.
    pub async fn login_messages_with_retry(&self) -> Result<(String, String, i64)> {
        let mut auth = self.auth.lock().await.clone();
        
        // Helper to update the locked state
        let update_auth = |new_auth: AuthState| async {
            let mut lock = self.auth.lock().await;
            *lock = new_auth;
        };

        // Try 1: Just login
        match super::messages_auth::login_messages(&auth).await {
            Ok(res) => return Ok(res),
            Err(e) => {
                let err_str = e.to_string();
                if !err_str.contains("AutoLoginToken not found") && !err_str.contains("TokenIsExpired") {
                    return Err(e);
                }
                println!("[Client] Messages login failed ({}). Attempting refresh...", err_str);
            }
        }

        // Try 2: Refresh Synergia token
        match auth::refresh_synergia_token(&auth).await {
            Ok(new_auth) => {
                auth = new_auth.clone();
                update_auth(new_auth).await;
            }
            Err(e) => {
                println!("[Client] Synergia refresh failed: {}. Attempting full login...", e);
                // Try 3: Full login with saved credentials
                let email = auth.portal_email.clone();
                let password = auth.portal_password.clone();
                
                if email.is_empty() || password.is_empty() {
                    bail!("Cannot re-login: no saved credentials");
                }

                let new_auth = auth::login_portal(&email, &password).await?;
                // Preserve Synergia credentials if any
                let mut final_auth = new_auth;
                final_auth.synergia_username = auth.synergia_username.clone();
                final_auth.synergia_password = auth.synergia_password.clone();
                
                auth = final_auth.clone();
                update_auth(final_auth).await;
            }
        }

        // Final retry
        super::messages_auth::login_messages(&auth).await
    }

    /// Force refresh the messages session.
    pub async fn force_refresh_messages_session(&self) -> Result<String> {
        // Invalidate current session first
        {
            let mut auth = self.auth.lock().await;
            auth.messages_session_id = None;
            auth.messages_session_expiry = None;
        }
        
        // println!("[Client] Forcing refresh of Synergia session...");
        match self.login_messages_with_retry().await {
            Ok((msg_cookie, syn_cookie, expiry)) => {
                let mut auth = self.auth.lock().await;
                auth.messages_session_id = Some(msg_cookie.clone());
                auth.synergia_cookie = Some(syn_cookie);
                auth.messages_session_expiry = Some(expiry);
                Ok(msg_cookie)
            }
            Err(e) => Err(e),
        }
    }

    /// Helper to get or refresh Synergia session cookie
    async fn get_synergia_session(&self) -> Result<String> {
        // Check validity first (scoped lock)
        {
            let auth = self.auth.lock().await;
            let is_valid = if let Some(expiry) = auth.messages_session_expiry {
                 chrono::Utc::now().timestamp() < expiry
            } else {
                false
            };

            if is_valid {
                 if let Some(cookie) = &auth.synergia_cookie {
                     return Ok(cookie.clone());
                 }
            }
        }
        
        // Login again
        // println!("[Client] Refreshing Synergia session...");
        match self.login_messages_with_retry().await {
            Ok((msg_cookie, syn_cookie, expiry)) => {
                let mut auth = self.auth.lock().await;
                auth.messages_session_id = Some(msg_cookie);
                auth.synergia_cookie = Some(syn_cookie.clone());
                auth.messages_session_expiry = Some(expiry);
                Ok(syn_cookie)
            }
            Err(e) => Err(e),
        }
    }
}
