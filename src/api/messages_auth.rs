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
    // println!("[Messages Auth] Starting messages authentication via AutoLoginToken...");
    
    // Step 1: Get AutoLoginToken from API
    let client = Client::builder()
        .user_agent(LIBRUS_USER_AGENT)
        .build()
        .context("Failed to create HTTP client")?;
    
    // println!("[Messages Auth] Requesting AutoLoginToken from API...");
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
    
    // println!("[Messages Auth] Got AutoLoginToken: {}...", &auto_login_token[..20.min(auto_login_token.len())]);
    
    // Step 2: Use token to establish Synergia session and get cookies
    let jar = std::sync::Arc::new(reqwest::cookie::Jar::default());
    let client_with_cookies = Client::builder()
        .user_agent(LIBRUS_USER_AGENT)
        .cookie_provider(std::sync::Arc::clone(&jar))
        .redirect(reqwest::redirect::Policy::none()) // Manual redirect handling
        .build()
        .context("Failed to create HTTP client with cookies")?;
    
    // Step 2a: Establish Synergia session
    // We need to follow redirects manually until we hit a final page (likely uczen/index)
    let login_url = format!(
        "https://synergia.librus.pl/loguj/token/{}/przenies",
        auto_login_token
    );
    
    let mut current_url = login_url;
    // println!("[Messages Auth] Phase 1: Establishing Synergia session starting at: {}", current_url);

    loop {
        let response = client_with_cookies
            .get(&current_url)
            .send()
            .await
            .context("Failed to send request in Synergia login loop")?;
            
        let status = response.status();
        // println!("[Messages Auth] {} -> {}", current_url, status);

        if status.is_redirection() {
            if let Some(location_header) = response.headers().get("location") {
                let location = location_header.to_str()?.to_string();
                
                // Handle relative URLs
                if location.starts_with('/') {
                    let current_url_parsed = reqwest::Url::parse(&current_url)?;
                    let host = current_url_parsed.host_str().unwrap_or("synergia.librus.pl");
                    let scheme = current_url_parsed.scheme();
                    current_url = format!("{}://{}{}", scheme, host, location);
                } else {
                     current_url = location;
                }
                
                // println!("[Messages Auth] Following redirect to: {}", current_url);
                continue;
            }
        }
        
        // println!("[Messages Auth] Phase 1 complete. Landed at: {} ({})", current_url, status);
        break;
    }

    // Step 2b: Access messages module to trigger AutoLogon
    let messages_start_url = "https://synergia.librus.pl/wiadomosci2";
    current_url = messages_start_url.to_string();
    // println!("[Messages Auth] Phase 2: Accessing messages module: {}", current_url);
    
    let mut found_autologon = false;

    loop {
        let response = client_with_cookies
            .get(&current_url)
            .send()
            .await
            .context("Failed to send request in Messages login loop")?;
            
        let status = response.status();
        // println!("[Messages Auth] {} -> {}", current_url, status);

        if status.is_redirection() {
            if let Some(location_header) = response.headers().get("location") {
                let location = location_header.to_str()?.to_string();
                
                // Check for AutoLogon
                if location.contains("AutoLogon") {
                    // println!("[Messages Auth] Found AutoLogon in redirect: {}. Stopping to extract cookies.", location);
                    found_autologon = true;
                    // We DO NOT follow this redirect. The cookies should be in the jar now.
                    // Actually, usually cookies are set in the response THAT redirects to AutoLogon?
                    // Or AutoLogon URL itself contains the session ID? 
                    // No, usually the response setting the cookie redirects to AutoLogon (which validates it).
                    // Wait, reference says "saveSessionId(response, text)".
                    // If we stop here, we check the jar.
                    break;
                }
                
                // Handle relative URLs
                if location.starts_with('/') {
                    let current_url_parsed = reqwest::Url::parse(&current_url)?;
                    let host = current_url_parsed.host_str().unwrap_or("synergia.librus.pl");
                    let scheme = current_url_parsed.scheme();
                    current_url = format!("{}://{}{}", scheme, host, location);
                } else {
                     current_url = location;
                }
                
                // println!("[Messages Auth] Following redirect to: {}", current_url);
                continue;
            }
        }
        
        // println!("[Messages Auth] Phase 2 stopped (status: {}, no Location or not redirect).", status);
        break;
    }
    
    // Extract cookies from cookie jar (they're not in response headers!)
    let url_wiadomosci = "https://wiadomosci.librus.pl".parse::<reqwest::Url>()
        .context("Failed to parse wiadomosci URL")?;
    
    let cookies_header = jar.cookies(&url_wiadomosci)
        .ok_or_else(|| anyhow::anyhow!("No cookies found for wiadomosci.librus.pl"))?;
    
    let cookies_str = cookies_header.to_str()
        .context("Failed to convert cookies to string")?;
    
    // println!("[Messages Auth] Cookies for wiadomosci: {}", cookies_str);

    // Also extract cookies for Synergia!
    let url_synergia = "https://synergia.librus.pl".parse::<reqwest::Url>()
        .context("Failed to parse synergia URL")?;
    let synergia_cookies_header = jar.cookies(&url_synergia);
    let synergia_cookies_str = if let Some(h) = synergia_cookies_header {
         h.to_str().unwrap_or("").to_string()
    } else {
        "".to_string()
    };
    // println!("[Messages Auth] Cookies for synergia: {}", synergia_cookies_str);
    
    // Parse DZIENNIKSID from cookie string and sanitize it
    let mut dzienniksid = None;
    
    for cookie_pair in cookies_str.split(';') {
        let cookie_pair = cookie_pair.trim();
        if cookie_pair.starts_with("DZIENNIKSID=") {
            if let Some(value) = cookie_pair.strip_prefix("DZIENNIKSID=") {
                // Sanitize the session ID as per Android app reference
                let sanitized = value.replace("-MAINT", "").replace("MAINT", "");
                dzienniksid = Some(sanitized);
            }
        }
    }

    if let Some(dziennik_sid) = dzienniksid {
        // println!("[Messages Auth] DZIENNIKSID obtained: {}", dziennik_sid);
        
        let now = chrono::Utc::now().timestamp();
        let expiry = now + (45 * 60);
        
        // Create strict cookie string with ONLY DZIENNIKSID
        // The Android app reference focuses on this specific cookie, and others might be causing issues.
        let cookie_str = format!("DZIENNIKSID={}", dziennik_sid);
        // println!("[Messages Auth] Final cookie string: {}", cookie_str);
        
        return Ok((cookie_str, synergia_cookies_str, expiry));
    }

    Err(anyhow::anyhow!("DZIENNIKSID cookie not found in cookie jar"))
}
