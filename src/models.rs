use axum::http::StatusCode;
use axum::response::Json;
use serde::Serialize;
use std::collections::HashMap;

#[derive(Serialize)]
pub struct ActionResponse {
    pub success: bool,
    pub message: String,
    pub stdout: String,
    pub stderr: String,
}

impl ActionResponse {
    pub fn ok(message: String) -> (StatusCode, Json<Self>) {
        (
            StatusCode::OK,
            Json(Self {
                success: true,
                message,
                stdout: String::new(),
                stderr: String::new(),
            }),
        )
    }
    pub fn err(status: StatusCode, message: &str) -> (StatusCode, Json<Self>) {
        (
            status,
            Json(Self {
                success: false,
                message: message.to_string(),
                stdout: String::new(),
                stderr: String::new(),
            }),
        )
    }
}

#[derive(Serialize)]
pub struct SystemStats {
    pub timestamp: String,
    pub memory: MemoryStats,
    pub cpu: CpuStats,
    pub disk: DiskStats,
    pub uptime: UptimeStats,
    pub network: HashMap<String, NetworkStats>,
    pub services: HashMap<String, String>,
    pub load_avg: LoadAvgStats,
    pub temperature: f32,
}

#[derive(Serialize)]
pub struct MemoryStats {
    pub total: u64,
    pub used: u64,
    pub available: u64,
    pub percent: u64,
}

#[derive(Serialize)]
pub struct CpuStats {
    pub percent: f32,
    pub model: String,
    pub cores: usize,
}

#[derive(Serialize)]
pub struct DiskStats {
    pub total: u64,
    pub used: u64,
    pub available: u64,
    pub percent: u64,
}

#[derive(Serialize)]
pub struct UptimeStats {
    pub seconds: u64,
    pub days: u64,
    pub hours: u64,
    pub minutes: u64,
}

#[derive(Serialize)]
pub struct NetworkStats {
    pub rx: u64,
    pub tx: u64,
}

#[derive(Serialize)]
pub struct LoadAvgStats {
    pub one: f64,
    pub five: f64,
    pub fifteen: f64,
}
