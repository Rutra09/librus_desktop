use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Session {
    pub access_token: String,
    pub expires_at: i64,
    pub refresh_token: String,
    pub portal_access_token: String,
    pub portal_refresh_token: String,
    pub portal_expires_at: i64,
    pub user_id: i64,
    pub portal_email: String,
    pub portal_password: String,
    pub synergia_login: String,
    pub synergia_username: Option<String>,
    pub synergia_password: Option<String>,
    pub messages_session_id: Option<String>,
    pub messages_session_expiry: Option<i64>,
}

pub fn get_session_path() -> Result<PathBuf> {
    let project_dirs = ProjectDirs::from("com", "librus_desktop", "librus-front")
        .context("Could not determine project directories")?;
    let config_dir = project_dirs.config_dir();
    if !config_dir.exists() {
        fs::create_dir_all(config_dir)?;
    }
    Ok(config_dir.join("session.json"))
}

pub fn save_session(session: &Session) -> Result<()> {
    let path = get_session_path()?;
    let json = serde_json::to_string_pretty(session)?;
    fs::write(path, json)?;
    Ok(())
}

pub fn load_session() -> Result<Option<Session>> {
    let path = get_session_path()?;
    if !path.exists() {
        return Ok(None);
    }
    let json = fs::read_to_string(path)?;
    let session: Session = serde_json::from_str(&json)?;
    Ok(Some(session))
}

pub fn clear_session() -> Result<()> {
    let path = get_session_path()?;
    if path.exists() {
        fs::remove_file(path)?;
    }
    Ok(())
}
