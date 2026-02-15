//! Debug test for SynergiaAccounts endpoint

use librus_front::api::constants::*;

#[tokio::test]
#[ignore]
async fn debug_synergia_accounts() {
    let email = std::env::var("LIBRUS_EMAIL").unwrap();
    let password = std::env::var("LIBRUS_PASSWORD").unwrap();

    // Step 1: Do the full portal login to get portal_access_token
    let client = reqwest::Client::builder()
        .user_agent(LIBRUS_USER_AGENT)
        .redirect(reqwest::redirect::Policy::none())
        .cookie_store(true)
        .build()
        .unwrap();

    // GET authorize
    println!("=== Step 1: GET authorize ===");
    let mut url = LIBRUS_AUTHORIZE_URL.to_string();

    // Follow redirects to get login page
    loop {
        let response = client
            .get(&url)
            .header("X-Requested-With", LIBRUS_HEADER)
            .send()
            .await
            .unwrap();

        let status = response.status();
        println!("  {} -> {}", url, status);

        if let Some(location) = response.headers().get("location").and_then(|v| v.to_str().ok()) {
            if location.starts_with("app://librus") {
                println!("  Got auth code in redirect!");
                break;
            }
            url = if location.starts_with('/') {
                format!("https://portal.librus.pl{}", location)
            } else {
                location.to_string()
            };
            continue;
        }

        // Parse login page
        let body = response.text().await.unwrap();

        // Extract CSRF token
        let csrf = extract_meta_csrf(&body);
        println!("  CSRF: {:?}", csrf);

        // Extract hidden inputs
        let hiddens = extract_hidden_fields(&body);
        println!("  Hidden fields: {:?}", hiddens);

        // POST login
        println!("\n=== Step 2: POST login ===");
        let mut form: Vec<(String, String)> = hiddens;
        form.push(("email".to_string(), email.clone()));
        form.push(("password".to_string(), password.clone()));

        let mut req = client
            .post(LIBRUS_LOGIN_URL)
            .header("X-Requested-With", LIBRUS_HEADER)
            .header("Referer", &url);

        if let Some(ref token) = csrf {
            req = req.header("X-CSRF-TOKEN", token);
        }

        let login_resp = req.form(&form).send().await.unwrap();
        println!("  Status: {}", login_resp.status());

        if let Some(loc) = login_resp.headers().get("location").and_then(|v| v.to_str().ok()) {
            url = if loc.starts_with('/') {
                format!("https://portal.librus.pl{}", loc)
            } else {
                loc.to_string()
            };
            println!("  Redirect: {}", url);
        } else {
            let text = login_resp.text().await.unwrap();
            println!("  Body (first 500): {}", &text[..text.len().min(500)]);
            url = LIBRUS_AUTHORIZE_URL.to_string();
        }

        break;
    }

    // Follow redirects to get the code
    let mut code: Option<String> = None;
    for i in 0..10 {
        println!("\n=== Redirect {} ===", i);
        if url.starts_with("app://librus") {
            if let Some(q) = url.split('?').nth(1) {
                for param in q.split('&') {
                    if let Some(c) = param.strip_prefix("code=") {
                        code = Some(c.to_string());
                    }
                }
            }
            println!("  Got code: {:?}", code);
            break;
        }

        let full = if url.starts_with('/') {
            format!("https://portal.librus.pl{}", url)
        } else {
            url.clone()
        };

        let resp = client
            .get(&full)
            .header("X-Requested-With", LIBRUS_HEADER)
            .send()
            .await
            .unwrap();

        println!("  {} -> {}", full, resp.status());

        match resp.headers().get("location").and_then(|v| v.to_str().ok()) {
            Some(loc) => {
                url = loc.to_string();
                println!("  -> {}", url);
            }
            None => {
                let text = resp.text().await.unwrap();
                println!("  No redirect, body (first 300): {}", &text[..text.len().min(300)]);
                break;
            }
        }
    }

    let code = code.expect("Failed to get auth code");

    // Exchange code for portal token
    println!("\n=== Step 3: Exchange code for token ===");
    let token_client = reqwest::Client::builder()
        .user_agent(LIBRUS_USER_AGENT)
        .build()
        .unwrap();

    let params = [
        ("client_id", LIBRUS_CLIENT_ID),
        ("grant_type", "authorization_code"),
        ("code", code.as_str()),
        ("redirect_uri", LIBRUS_REDIRECT_URL),
    ];

    let resp = token_client
        .post(LIBRUS_TOKEN_URL)
        .form(&params)
        .send()
        .await
        .unwrap();

    println!("  Status: {}", resp.status());
    let token_body = resp.text().await.unwrap();
    println!("  Body: {}", token_body);

    let token_json: serde_json::Value = serde_json::from_str(&token_body).unwrap();
    let portal_access_token = token_json["access_token"].as_str().unwrap();
    println!("  Portal token: {}...", &portal_access_token[..20.min(portal_access_token.len())]);

    // Step 4: GET SynergiaAccounts
    println!("\n=== Step 4: GET SynergiaAccounts ===");
    let accounts_url = format!("{}{}", LIBRUS_PORTAL_URL, LIBRUS_ACCOUNTS_URL);
    println!("  URL: {}", accounts_url);

    let resp = token_client
        .get(&accounts_url)
        .header("Authorization", format!("Bearer {}", portal_access_token))
        .send()
        .await
        .unwrap();

    println!("  Status: {}", resp.status());
    let accounts_body = resp.text().await.unwrap();
    println!("  Body: {}", accounts_body);

    // Step 5: Try SynergiaAccounts/fresh
    let accounts_json: serde_json::Value = serde_json::from_str(&accounts_body).unwrap();
    println!("  JSON type: {}", if accounts_json.is_array() { "array" } else if accounts_json.is_object() { "object" } else { "other" });

    // If it's an object, try to find accounts in it
    if let Some(obj) = accounts_json.as_object() {
        for (key, value) in obj {
            println!("  Key: {} -> type: {}", key, 
                if value.is_array() { "array" } else if value.is_object() { "object" } else if value.is_string() { "string" } else { "other" });
        }
    }

    // Try to extract login from accounts response
    if let Some(arr) = accounts_json.as_array() {
        for acc in arr {
            println!("  Account: {}", acc);
        }
    }

    // Also try the "accounts" or similar key
    for key in ["accounts", "Accounts", "synergiaAccounts", "SynergiaAccounts"] {
        if let Some(arr) = accounts_json.get(key) {
            println!("  Found key '{}': {}", key, arr);
        }
    }
}

fn extract_meta_csrf(html: &str) -> Option<String> {
    let pattern = r#"name="csrf-token" content=""#;
    if let Some(start) = html.find(pattern) {
        let after = &html[start + pattern.len()..];
        if let Some(end) = after.find('"') {
            return Some(after[..end].to_string());
        }
    }
    let pattern2 = r#"name="_token" value=""#;
    if let Some(start) = html.find(pattern2) {
        let after = &html[start + pattern2.len()..];
        if let Some(end) = after.find('"') {
            return Some(after[..end].to_string());
        }
    }
    None
}

fn extract_hidden_fields(html: &str) -> Vec<(String, String)> {
    let mut result = Vec::new();
    let lower = html.to_lowercase();
    let mut pos = 0;
    while let Some(start) = lower[pos..].find("<input") {
        let abs = pos + start;
        if let Some(end) = lower[abs..].find('>') {
            let tag = &html[abs..abs + end + 1];
            if tag.to_lowercase().contains("type=\"hidden\"") {
                let name = extract_attr(tag, "name");
                let value = extract_attr(tag, "value");
                if let (Some(n), Some(v)) = (name, value) {
                    result.push((n, v));
                }
            }
            pos = abs + end + 1;
        } else { break; }
    }
    result
}

fn extract_attr(tag: &str, attr: &str) -> Option<String> {
    let pat = format!("{}=\"", attr);
    let lower = tag.to_lowercase();
    if let Some(start) = lower.find(&pat) {
        let after = &tag[start + pat.len()..];
        if let Some(end) = after.find('"') {
            return Some(after[..end].to_string());
        }
    }
    None
}
