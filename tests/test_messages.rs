#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires valid credentials
    async fn test_messages_auth() {
        // This test requires valid Librus credentials
        // To run: cargo test test_messages_auth -- --ignored
        
        let email = std::env::var("LIBRUS_EMAIL").expect("Set LIBRUS_EMAIL env var");
        let password = std::env::var("LIBRUS_PASSWORD").expect("Set LIBRUS_PASSWORD env var");

        println!("Testing portal login...");
        let auth_state = librus_front::api::auth::login_portal(&email, &password)
            .await
            .expect("Portal login failed");

        println!("Portal login successful!");
        println!("Testing messages authentication...");
        
        let result = librus_front::api::messages_auth::login_messages(&auth_state).await;
        
        match result {
            Ok((session_id, expiry)) => {
                println!("Messages auth successful!");
                println!("Session ID: {}", session_id);
                println!("Expiry: {}", expiry);
                assert!(!session_id.is_empty());
                assert!(expiry > 0);
            }
            Err(e) => {
                eprintln!("Messages auth failed: {:?}", e);
                panic!("Messages authentication failed");
            }
        }
    }

    #[tokio::test]
    #[ignore] // Requires valid credentials
    async fn test_fetch_messages() {
        let email = std::env::var("LIBRUS_EMAIL").expect("Set LIBRUS_EMAIL env var");
        let password = std::env::var("LIBRUS_PASSWORD").expect("Set LIBRUS_PASSWORD env var");

        println!("Logging in...");
        let auth_state = librus_front::api::auth::login_portal(&email, &password)
            .await
            .expect("Login failed");

        let client = librus_front::api::client::LibrusClient::new(auth_state);

        println!("Fetching messages...");
        let result = client.fetch_messages().await;

        match result {
            Ok(messages) => {
                println!("Successfully fetched {} messages", messages.len());
                for (i, msg) in messages.iter().take(5).enumerate() {
                    println!("\nMessage {}:", i + 1);
                    println!("  ID: {}", msg.id);
                    println!("  Subject: {}", msg.subject);
                    println!("  Sender: {}", msg.sender_name);
                    println!("  Date: {}", msg.send_date);
                    println!("  Read: {}", msg.read_date.is_some());
                    println!("  Has attachments: {}", msg.has_attachments);
                }
            }
            Err(e) => {
                eprintln!("Failed to fetch messages: {:?}", e);
                panic!("Message fetch failed");
            }
        }
    }
}
