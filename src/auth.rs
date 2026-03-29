use axum::body::Body;
use axum::{
    http::HeaderMap, http::Request, http::StatusCode, middleware::Next, response::IntoResponse,
    response::Json, response::Response,
};
use base64::{Engine, engine::general_purpose};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};
use totp_rs::{Algorithm, Secret, TOTP};
use yescrypt::{PasswordHash, PasswordVerifier, Yescrypt};

static JWT_SECRET: OnceLock<String> = OnceLock::new();

const ROTATION_DAYS: u64 = 7;
const TOTP_SECRET_PATH: &str = "/var/lib/server-dash-api/google-auth/jack";

fn secret_path() -> PathBuf {
    PathBuf::from("/var/lib/server-dash-api/jwt_secret")
}

fn generate_secret() -> String {
    format!(
        "{:016x}{:016x}",
        rand::random::<u64>(),
        rand::random::<u64>()
    )
}

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

pub fn jwt_secret() -> &'static str {
    JWT_SECRET.get_or_init(|| {
        let path = secret_path();
        std::fs::create_dir_all(path.parent().unwrap()).ok();

        if let Ok(contents) = std::fs::read_to_string(&path) {
            if let Some((ts_str, secret)) = contents.trim().split_once(':') {
                if let Ok(ts) = ts_str.parse::<u64>() {
                    if current_timestamp() - ts < ROTATION_DAYS * 86400 {
                        return secret.to_string();
                    }
                    println!("JWT secret expired, rotating...");
                }
            }
        }

        let secret = generate_secret();
        let contents = format!("{}:{}", current_timestamp(), secret);
        std::fs::write(&path, &contents).ok();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).ok();
        }

        println!("Generated new JWT secret");
        secret
    })
}

#[derive(Serialize, Deserialize)]
struct Claims {
    sub: String,
    exp: usize,
}

pub fn create_token(username: &str) -> String {
    let claims = Claims {
        sub: username.to_owned(),
        exp: (chrono::Utc::now() + chrono::Duration::hours(8)).timestamp() as usize,
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(jwt_secret().as_bytes()),
    )
    .unwrap()
}

pub fn verify_token(headers: &HeaderMap) -> bool {
    let Some(val) = headers.get("Authorization") else {
        return false;
    };
    let token = val.to_str().unwrap_or("").replace("Bearer ", "");
    decode::<Claims>(
        &token,
        &DecodingKey::from_secret(jwt_secret().as_bytes()),
        &Validation::default(),
    )
    .is_ok()
}

pub fn decode_basic_auth(headers: &HeaderMap) -> Option<(String, String, String)> {
    let val = headers.get("Authorization")?.to_str().ok()?;
    let encoded = val.strip_prefix("Basic ")?;
    let decoded = general_purpose::STANDARD.decode(encoded).ok()?;
    let s = String::from_utf8(decoded).ok()?;
    let (user, rest) = s.split_once(':')?;
    if rest.len() < 6 {
        return None;
    }
    let (password, totp) = rest.split_at(rest.len() - 6);
    Some((user.to_string(), password.to_string(), totp.to_string()))
}

fn verify_password(username: &str, password: &str) -> bool {
    let shadow_content = match std::fs::read_to_string("/etc/shadow") {
        Ok(c) => c,
        Err(e) => {
            println!("Failed to read /etc/shadow: {}", e);
            return false;
        }
    };
    for line in shadow_content.lines() {
        let fields: Vec<&str> = line.split(':').collect();
        if fields.len() < 2 {
            continue;
        }
        if fields[0] != username {
            continue;
        }
        return verify_shadow_hash(password, fields[1]);
    }
    println!("User not found in shadow");
    false
}

fn verify_shadow_hash(password: &str, hash: &str) -> bool {
    let parsed_hash = match PasswordHash::new(hash) {
        Ok(h) => h,
        Err(e) => {
            println!("Failed to parse hash: {:?}", e);
            return false;
        }
    };
    Yescrypt::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok()
}

fn verify_totp(totp_code: &str) -> bool {
    let secret_file = match std::fs::read_to_string(TOTP_SECRET_PATH) {
        Ok(f) => f,
        Err(e) => {
            println!("Failed to read TOTP secret: {}", e);
            return false;
        }
    };

    let secret_b32 = secret_file.lines().next().unwrap_or("").trim().to_string();

    let totp = match TOTP::new(
        Algorithm::SHA1,
        6,
        1,
        30,
        Secret::Encoded(secret_b32).to_bytes().unwrap(),
        None,
        "jack".to_string(),
    ) {
        Ok(t) => t,
        Err(e) => {
            println!("Failed to create TOTP: {:?}", e);
            return false;
        }
    };

    totp.check_current(totp_code).unwrap_or(false)
}

pub fn verify_system_credentials(username: &str, password: &str, totp: &str) -> bool {
    verify_password(username, password) && verify_totp(totp)
}

pub async fn require_auth(headers: HeaderMap, request: Request<Body>, next: Next) -> Response {
    if verify_token(&headers) {
        next.run(request).await
    } else {
        (StatusCode::UNAUTHORIZED, "Unauthorized").into_response()
    }
}

// POST /auth/login
pub async fn post_login(headers: HeaderMap) -> impl IntoResponse {
    let (username, password, totp) = match decode_basic_auth(&headers) {
        Some(c) => c,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                "Missing or invalid Authorization header",
            )
                .into_response();
        }
    };

    if !verify_system_credentials(&username, &password, &totp) {
        return (StatusCode::UNAUTHORIZED, "Invalid credentials").into_response();
    }

    let token = create_token(&username);
    (StatusCode::OK, Json(serde_json::json!({ "token": token }))).into_response()
}
