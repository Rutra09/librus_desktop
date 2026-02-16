use anyhow::{Context, Result};
use reqwest::Client;
use reqwest::cookie::CookieStore;
use super::constants::*;

/// Authenticate with the Messages module to get DZIENNIKSID cookie.
/// 
/// Uses the API AutoLoginToken endpoint to get a Synergia token, then uses that token
/// to access Synergia and get the messages session cookie.
/// Returns (messages_cookie, synergia_cookie, expiry)
pub async fn login_messages(auth: &super::auth::AuthState) -> Result<(String, String, i64)> {
    println!("[Messages Auth] Starting messages authentication via AutoLoginToken...");
    
    // Step 1: Get AutoLoginToken from API
    let client = Client::builder()
        .user_agent(SYNERGIA_USER_AGENT)
        .build()
        .context("Failed to create HTTP client")?;
    
    println!("[Messages Auth] Requesting AutoLoginToken from API...");
    let token_response = client
        .post(&format!("{}/AutoLoginToken", LIBRUS_API_URL))
        .header("Authorization", format!("Bearer {}", &auth.access_token))
        .send()
        .await
        .context("Failed to request AutoLoginToken")?;
    
    let token_json: serde_json::Value = token_response.json().await
        .context("Failed to parse AutoLoginToken response")?;
    
    let auto_login_token = token_json["Token"].as_str()
        .ok_or_else(|| anyhow::anyhow!("AutoLoginToken not found in response"))?;
    
    println!("[Messages Auth] Got AutoLoginToken: {}...", &auto_login_token[..20.min(auto_login_token.len())]);
    
    // Step 2: Use token to establish Synergia session and get cookies
    let jar = std::sync::Arc::new(reqwest::cookie::Jar::default());
    let client_with_cookies = Client::builder()
        .user_agent(LIBRUS_USER_AGENT)
        .cookie_provider(std::sync::Arc::clone(&jar))
        .build() // Allow redirects to set cookies properly
        .context("Failed to create HTTP client with cookies")?;
    
    // Step 2a: First, use token to establish Synergia session
    let synergia_url = format!(
        "https://synergia.librus.pl/loguj/token/{}/przenies",
        auto_login_token
    );
    
    println!("[Messages Auth] Step 1: Establishing Synergia session: {}", synergia_url);
    let synergia_response = client_with_cookies
        .get(&synergia_url)
        .send()
        .await
        .context("Failed to establish Synergia session")?;
    
    println!("[Messages Auth] Synergia response status: {}", synergia_response.status());
    
    // Step 2b: Now access /wiadomosci2 with the established session
    let messages_url = "https://synergia.librus.pl/wiadomosci2";
    println!("[Messages Auth] Step 2: Accessing messages: {}", messages_url);
    
    let messages_response = client_with_cookies
        .get(messages_url)
        .send()
        .await
        .context("Failed to access messages")?;
    
    println!("[Messages Auth] Messages response status: {}", messages_response.status());
    
    // Extract cookies from cookie jar (they're not in response headers!)
    let url_wiadomosci = "https://wiadomosci.librus.pl".parse::<reqwest::Url>()
        .context("Failed to parse wiadomosci URL")?;
    
    let cookies_header = jar.cookies(&url_wiadomosci)
        .ok_or_else(|| anyhow::anyhow!("No cookies found for wiadomosci.librus.pl"))?;
    
    let cookies_str = cookies_header.to_str()
        .context("Failed to convert cookies to string")?;
    
    println!("[Messages Auth] Cookies for wiadomosci: {}", cookies_str);

    // Also extract cookies for Synergia!
    let url_synergia = "https://synergia.librus.pl".parse::<reqwest::Url>()
        .context("Failed to parse synergia URL")?;
    let synergia_cookies_header = jar.cookies(&url_synergia);
    let synergia_cookies_str = if let Some(h) = synergia_cookies_header {
         h.to_str().unwrap_or("").to_string()
    } else {
        "".to_string()
    };
    println!("[Messages Auth] Cookies for synergia: {}", synergia_cookies_str);
    
    // Parse DZIENNIKSID and cookiesession1 from cookie string
    let mut dzienniksid = None;
    let mut cookiesession1 = None;
    
    for cookie_pair in cookies_str.split(';') {
        let cookie_pair = cookie_pair.trim();
        if cookie_pair.starts_with("DZIENNIKSID=") {
            if let Some(value) = cookie_pair.strip_prefix("DZIENNIKSID=") {
                dzienniksid = Some(value.to_string());
            }
        } else if cookie_pair.starts_with("cookiesession1=") {
            if let Some(value) = cookie_pair.strip_prefix("cookiesession1=") {
                cookiesession1 = Some(value.to_string());
            }
        }
    }

    if let Some(dziennik_sid) = dzienniksid {
        println!("[Messages Auth] DZIENNIKSID obtained: {}", dziennik_sid);
        
        let now = chrono::Utc::now().timestamp();
        let expiry = now + (45 * 60);
        
        // Build cookie string
        let cookie_str = if let Some(cookie_session) = cookiesession1 {
            println!("[Messages Auth] cookiesession1: {}", cookie_session);
            format!("DZIENNIKSID={}; cookiesession1={}", dziennik_sid, cookie_session)
        } else {
            format!("DZIENNIKSID={}", dziennik_sid)
        };
        
        return Ok((cookie_str, synergia_cookies_str, expiry));
    }

    Err(anyhow::anyhow!("DZIENNIKSID cookie not found in cookie jar"))
}
