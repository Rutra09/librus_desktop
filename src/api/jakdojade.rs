use anyhow::{Context, Result};
use base64::Engine;
use directories::ProjectDirs;
use hmac::{Hmac, Mac};
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, CONTENT_TYPE, USER_AGENT};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256, Sha512};
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;

type HmacSha512 = Hmac<Sha512>;

fn base64_url(input: &[u8]) -> String {
    let b64 = base64::engine::general_purpose::STANDARD.encode(input);
    b64.replace('+', "-").replace('/', "_").replace('=', "")
}

fn sha256_base64_url(data: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data.as_bytes());
    let result = hasher.finalize();
    base64_url(&result)
}

fn hmac_sha512_base64_url(key: &str, data: &str) -> String {
    let mut mac = HmacSha512::new_from_slice(key.as_bytes()).expect("HMAC key length");
    mac.update(data.as_bytes());
    let result = mac.finalize().into_bytes();
    base64_url(&result)
}

pub fn generate_signature(
    path: &str,
    body_json: Option<&str>,
    params_str: Option<&str>,
    profile_login: &str,
    password_hash: &str,
    timestamp: &str,
) -> String {
    let body_hash = match body_json {
        Some(b) if !b.is_empty() => sha256_base64_url(b),
        _ => String::new(),
    };
    let params_hash = match params_str {
        Some(p) if !p.is_empty() => sha256_base64_url(p),
        _ => String::new(),
    };

    let payload = format!(
        "{}_{}_{}_{}_{}",
        path.to_lowercase(),
        timestamp,
        profile_login,
        body_hash,
        params_hash
    );

    hmac_sha512_base64_url(password_hash, &payload)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationConfig {
    pub city_symbol: String,
    pub location_name: String,
    pub location_type: String, // "ADDRESS" or "STOP_POINT"
    pub location_code: String,
    pub y_lat: f64,
    pub x_lon: f64,
}

impl Default for LocationConfig {
    fn default() -> Self {
        Self {
            city_symbol: "WARSZAWA".to_string(),
            location_name: "Plac Defilad 1".to_string(),
            location_type: "ADDRESS".to_string(),
            location_code: "".to_string(),
            y_lat: 52.2319,
            x_lon: 21.0067,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JakdojadeConfig {
    pub enabled: bool,
    pub profile_login: String,
    pub password_hash: String,
    pub device_id: String,
    pub start_location: LocationConfig,
    pub dest_location: LocationConfig,
    pub buffer_minutes: i32,
    #[serde(default)]
    pub avoid_lines: String,
    #[serde(default)]
    pub avoid_changes: bool,
    #[serde(default)]
    pub prefer_metro: bool,
    #[serde(default)]
    pub preferred_lines: String,
}

impl Default for JakdojadeConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            profile_login: "".to_string(),
            password_hash: "".to_string(),
            device_id: format!("device-web-{}", Uuid::new_v4()),
            start_location: LocationConfig {
                city_symbol: "WARSZAWA".to_string(),
                location_name: "Plac Defilad 1".to_string(),
                location_type: "ADDRESS".to_string(),
                location_code: "".to_string(),
                y_lat: 52.2319,
                x_lon: 21.0067,
            },
            dest_location: LocationConfig {
                city_symbol: "WARSZAWA".to_string(),
                location_name: "Międzynarodowa".to_string(),
                location_type: "STOP_POINT".to_string(),
                location_code: "209802".to_string(),
                y_lat: 52.2299,
                x_lon: 21.0687,
            },
            buffer_minutes: 5,
            avoid_lines: "".to_string(),
            avoid_changes: false,
            prefer_metro: false,
            preferred_lines: "".to_string(),
        }
    }
}


pub fn get_config_path() -> Result<PathBuf> {
    let project_dirs = ProjectDirs::from("com", "librus_desktop", "librus-front")
        .context("Could not determine project directories")?;
    let config_dir = project_dirs.config_dir();
    if !config_dir.exists() {
        fs::create_dir_all(config_dir)?;
    }
    Ok(config_dir.join("jakdojade_config.json"))
}

pub fn save_config(config: &JakdojadeConfig) -> Result<()> {
    let path = get_config_path()?;
    let json = serde_json::to_string_pretty(config)?;
    fs::write(path, json)?;
    Ok(())
}

pub fn load_config() -> Result<JakdojadeConfig> {
    let path = get_config_path()?;
    if !path.exists() {
        let default_cfg = JakdojadeConfig::default();
        let _ = save_config(&default_cfg);
        return Ok(default_cfg);
    }
    let json = fs::read_to_string(path)?;
    let config: JakdojadeConfig = serde_json::from_str(&json)?;
    Ok(config)
}

#[derive(Debug, Clone)]
pub struct ParsedRouteResult {
    pub departure_time: String, // "07:25"
    pub arrival_time: String,   // "07:52"
    pub total_minutes: i32,     // 27
    pub changes: i32,           // 0
    pub lines_text: String,     // "516 -> 509"
    pub summary_text: String,   // "Pieszo 4m -> 516 -> Pieszo 5m -> 509"
}


#[derive(Clone)]
pub struct JakdojadeClient {
    http: reqwest::Client,
    config: JakdojadeConfig,
}

impl JakdojadeClient {
    pub fn new(config: JakdojadeConfig) -> Self {
        Self {
            http: reqwest::Client::new(),
            config,
        }
    }

    pub fn get_config(&self) -> &JakdojadeConfig {
        &self.config
    }

    fn build_base_headers(device_id: &str, timestamp: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            USER_AGENT,
            HeaderValue::from_static(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:129.0) Gecko/20100101 Firefox/129.0",
            ),
        );
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(
            "X-jd-param-app-platform",
            HeaderValue::from_static("web"),
        );
        headers.insert(
            "X-jd-param-app-version",
            HeaderValue::from_static("1.0.0"),
        );
        headers.insert("X-jd-param-locale", HeaderValue::from_static("en"));
        headers.insert("X-jd-security-version", HeaderValue::from_static("4"));
        headers.insert(
            "X-jd-ticket-system-version",
            HeaderValue::from_static("28"),
        );
        headers.insert(ACCEPT, HeaderValue::from_static("application/json"));
        if let Ok(v) = HeaderValue::from_str(device_id) {
            headers.insert("X-jd-param-user-device-id", v);
        }
        if let Ok(v) = HeaderValue::from_str(timestamp) {
            headers.insert("X-jd-timestamp", v);
        }
        headers
    }

    pub async fn register_anonymous(&mut self) -> Result<()> {
        if !self.config.profile_login.is_empty() && !self.config.password_hash.is_empty() {
            return Ok(());
        }

        let timestamp = chrono::Utc::now().timestamp().to_string();
        let headers = Self::build_base_headers(&self.config.device_id, &timestamp);
        let url = "https://api.jakdojade.pl/api/profiles/v2/register-anonymous";

        let res = self
            .http
            .post(url)
            .headers(headers)
            .json(&serde_json::json!({}))
            .send()
            .await?;

        if !res.status().is_success() {
            let body = res.text().await.unwrap_or_default();
            anyhow::bail!("Jakdojade registration failed: {}", body);
        }

        let json: serde_json::Value = res.json().await?;
        if let (Some(login), Some(hash)) = (
            json["profileLogin"].as_str(),
            json["passwordHash"].as_str(),
        ) {
            self.config.profile_login = login.to_string();
            self.config.password_hash = hash.to_string();
            save_config(&self.config)?;
            log::info!("Registered new anonymous Jakdojade device: {}", login);
        }
        Ok(())
    }

    pub async fn find_route(
        &mut self,
        target_arrival_iso: &str, // e.g. "2026-09-02T08:00:00.000Z"
    ) -> Result<ParsedRouteResult> {
        self.register_anonymous().await?;

        let timestamp = chrono::Utc::now().timestamp().to_string();
        let endpoint = "/api/jd/v3/routes";
        let url = format!("https://api.jakdojade.pl{}", endpoint);

        let avoid_lines_vec: Vec<String> = self
            .config
            .avoid_lines
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        let avoid_changes_str = if self.config.avoid_changes {
            "AVOID_CHANGES"
        } else {
            "DEFAULT"
        };

        let mut preferred_lines_vec: Vec<String> = self
            .config
            .preferred_lines
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        if self.config.prefer_metro {
            if !preferred_lines_vec.iter().any(|l| l.eq_ignore_ascii_case("M1")) {
                preferred_lines_vec.push("M1".to_string());
            }
            if !preferred_lines_vec.iter().any(|l| l.eq_ignore_ascii_case("M2")) {
                preferred_lines_vec.push("M2".to_string());
            }
        }

        let payload = serde_json::json!({
            "engine": "DEFAULT",
            "fetchType": "SYNC",
            "routesCorrelation": "NONE",
            "userLocation": null,
            "searchQuery": {
                "start": {
                    "citySymbol": self.config.start_location.city_symbol,
                    "coordinate": {
                        "y_lat": self.config.start_location.y_lat,
                        "x_lon": self.config.start_location.x_lon
                    },
                    "locationType": self.config.start_location.location_type,
                    "locationName": self.config.start_location.location_name,
                    "locationCode": self.config.start_location.location_code
                },
                "destination": {
                    "citySymbol": self.config.dest_location.city_symbol,
                    "coordinate": {
                        "y_lat": self.config.dest_location.y_lat,
                        "x_lon": self.config.dest_location.x_lon
                    },
                    "locationType": self.config.dest_location.location_type,
                    "locationName": self.config.dest_location.location_name,
                    "locationCode": self.config.dest_location.location_code
                },
                "timeOptions": {
                    "dateTime": target_arrival_iso,
                    "queryTimeType": "ARRIVAL"
                },
                "realtimeSearchMode": "REALTIME_ENABLED",
                "routesCount": 3,
                "userConnectionTypePreference": "OPTIMAL",
                "publicTransportOptions": {
                    "avoidChanges": avoid_changes_str,
                    "avoidVehicles": [],
                    "prohibitedVehicles": [],
                    "prohibitedOperators": [],
                    "avoidLineTypes": [],
                    "accessibilityOptions": "NONE",
                    "preferredLines": preferred_lines_vec,
                    "avoidLines": avoid_lines_vec,
                    "forcedChangeTime": null
                }
            }
        });


        let body_str = serde_json::to_string(&payload)?;
        let signature = generate_signature(
            endpoint,
            Some(&body_str),
            None,
            &self.config.profile_login,
            &self.config.password_hash,
            &timestamp,
        );

        let mut headers = Self::build_base_headers(&self.config.device_id, &timestamp);
        headers.insert(
            "X-jd-param-profile-login",
            HeaderValue::from_str(&self.config.profile_login)?,
        );
        headers.insert("X-jd-sign", HeaderValue::from_str(&signature)?);

        let res = self
            .http
            .post(&url)
            .headers(headers)
            .body(body_str)
            .send()
            .await?;

        if !res.status().is_success() {
            let err_text = res.text().await.unwrap_or_default();
            anyhow::bail!("Jakdojade route search error: {}", err_text);
        }

        let json: serde_json::Value = res.json().await?;
        let routes = json["routes"]
            .as_array()
            .context("No routes field in response")?;

        if routes.is_empty() {
            anyhow::bail!("No routes found for specified locations");
        }

        let route = &routes[0];
        let parts = route["routeParts"]
            .as_array()
            .context("No routeParts found")?;

        if parts.is_empty() {
            anyhow::bail!("Route has no parts");
        }

        // Extract departure & arrival times
        let first_part = &parts[0];
        let last_part = &parts[parts.len() - 1];

        let dep_time_raw = first_part["startDeparture"]["dateTime"]
            .as_str()
            .or_else(|| first_part["startDeparture"].as_str())
            .unwrap_or("");
        let arr_time_raw = last_part["targetArrival"]["dateTime"]
            .as_str()
            .or_else(|| last_part["targetArrival"].as_str())
            .unwrap_or("");

        let format_time = |raw: &str| -> String {
            if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(raw) {
                let local_dt: chrono::DateTime<chrono::Local> = dt.into();
                return local_dt.format("%H:%M").to_string();
            }
            if let Ok(ndt) = chrono::NaiveDateTime::parse_from_str(raw, "%Y-%m-%dT%H:%M:%S%.fZ")
                .or_else(|_| chrono::NaiveDateTime::parse_from_str(raw, "%Y-%m-%dT%H:%M:%S"))
            {
                return ndt.format("%H:%M").to_string();
            }
            if raw.contains('T') {
                if let Some(t_part) = raw.split('T').nth(1) {
                    if t_part.len() >= 5 {
                        return t_part[0..5].to_string();
                    }
                }
            }
            raw.to_string()
        };

        let departure_time = format_time(dep_time_raw);
        let arrival_time = format_time(arr_time_raw);

        // Build route segments summary and vehicle lines list
        let mut segments = Vec::new();
        let mut vehicle_lines = Vec::new();
        let mut changes = 0;

        for part in parts {
            let part_type = part["routePartType"].as_str().unwrap_or("");
            if part_type == "WALK" {
                let duration_sec = part["durationSeconds"].as_i64().unwrap_or(0);
                let dur_min = (duration_sec as f32 / 60.0).round() as i32;
                if dur_min > 0 {
                    segments.push(format!("Pieszo {}m", dur_min));
                }
            } else if part_type == "VEHICLE_TRANSPORT" {
                let line_name = part["routeVehicle"]["routeLine"]["line"]["lineDisplayName"]["lineName"]
                    .as_str()
                    .or_else(|| part["routeVehicle"]["routeLine"]["line"]["lineDisplayName"]["name"].as_str())
                    .unwrap_or("?");

                segments.push(line_name.to_string());
                vehicle_lines.push(line_name.to_string());
                changes += 1;
            }
        }

        let changes_count = (changes as i32 - 1).max(0);
        let lines_text = if vehicle_lines.is_empty() {
            "Trasa piesza".to_string()
        } else {
            vehicle_lines.join(" -> ")
        };

        let summary_text = if segments.is_empty() {
            "Trasa piesza".to_string()
        } else {
            segments.join(" -> ")
        };




        // Calculate total minutes
        let total_minutes = if let (Ok(dep_dt), Ok(arr_dt)) = (
            chrono::NaiveDateTime::parse_from_str(dep_time_raw, "%Y-%m-%dT%H:%M:%S%.fZ")
                .or_else(|_| chrono::NaiveDateTime::parse_from_str(dep_time_raw, "%Y-%m-%dT%H:%M:%S")),
            chrono::NaiveDateTime::parse_from_str(arr_time_raw, "%Y-%m-%dT%H:%M:%S%.fZ")
                .or_else(|_| chrono::NaiveDateTime::parse_from_str(arr_time_raw, "%Y-%m-%dT%H:%M:%S")),
        ) {
            (arr_dt - dep_dt).num_minutes() as i32
        } else {
            0
        };

        Ok(ParsedRouteResult {
            departure_time,
            arrival_time,
            total_minutes,
            changes: changes_count,
            lines_text,
            summary_text,
        })

    }
}

pub async fn geocode_address(city: &str, address: &str) -> Option<(f64, f64)> {
    let query = format!("{}, {}", address.trim(), city.trim());
    if query.trim().len() < 3 {
        return None;
    }
    let client = reqwest::Client::new();
    let fmt = "json".to_string();
    let limit = "1".to_string();
    let url = reqwest::Url::parse_with_params(
        "https://nominatim.openstreetmap.org/search",
        &[("q", &query), ("format", &fmt), ("limit", &limit)],
    )
    .ok()?;


    let res = client
        .get(url)
        .header("User-Agent", "LibrusFrontApp/1.0")
        .send()
        .await
        .ok()?;

    if !res.status().is_success() {
        return None;
    }

    let json: serde_json::Value = res.json().await.ok()?;
    let first = json.as_array()?.first()?;
    let lat = first["lat"].as_str()?.parse::<f64>().ok()?;
    let lon = first["lon"].as_str()?.parse::<f64>().ok()?;

    Some((lat, lon))
}

