use axum::response::Redirect;
use axum::{
    Router, extract::Path, http::HeaderMap, http::StatusCode, response::IntoResponse,
    response::Json, routing::get, routing::post,
};
use base64::{Engine, engine::general_purpose};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use pam::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use sysinfo::{Components, Disks, Networks, System};
use tokio::process::Command;
use zbus::Connection;

const JWT_SECRET: &str = "change-me-to-a-long-random-string";

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
        &EncodingKey::from_secret(JWT_SECRET.as_bytes()),
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
        &DecodingKey::from_secret(JWT_SECRET.as_bytes()),
        &Validation::default(),
    )
    .is_ok()
}

pub fn decode_basic_auth(headers: &HeaderMap) -> Option<(String, String)> {
    let val = headers.get("Authorization")?.to_str().ok()?;
    let encoded = val.strip_prefix("Basic ")?;
    let decoded = general_purpose::STANDARD.decode(encoded).ok()?;
    let s = String::from_utf8(decoded).ok()?;
    let (user, pass) = s.split_once(':')?;
    Some((user.to_string(), pass.to_string()))
}

pub fn verify_system_credentials(username: &str, password: &str) -> bool {
    let mut client = match Client::with_password("login") {
        Ok(c) => c,
        Err(_) => return false,
    };
    client
        .conversation_mut()
        .set_credentials(username, password);
    client.authenticate().is_ok()
}

// POST /auth/login
pub async fn post_login(headers: HeaderMap) -> impl IntoResponse {
    let (username, password) = match decode_basic_auth(&headers) {
        Some(c) => c,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                "Missing or invalid Authorization header",
            )
                .into_response();
        }
    };
    if !verify_system_credentials(&username, &password) {
        return (StatusCode::UNAUTHORIZED, "Invalid credentials").into_response();
    }
    let token = create_token(&username);
    (StatusCode::OK, Json(serde_json::json!({ "token": token }))).into_response()
}
