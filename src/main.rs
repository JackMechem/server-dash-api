use axum::{Router, body::Body, response::Json, routing::get};
use serde_json::{Value, json};
use std::collections::HashMap;
use sysinfo::{Components, Disks, Networks, System};
use std::path::Path;
use tokio::process::Command;

mod models;

use models::SystemStats;

use self::models::MemoryStats;

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/", get(root))
        .route("/stats", get(get_stats));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3001").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

// which calls one of these handlers
async fn root() {}
async fn get_stats() -> Json<models::SystemStats> {
    let mut sys = System::new_all();
    sys.refresh_all();

    // Memory (MB)
    let memory = models::MemoryStats {
        total: sys.total_memory() / 1_000_000,
        used: sys.used_memory() / 1_000_000,
        available: sys.available_memory() / 1_000_000,
        percent: (sys.used_memory() as f64 / sys.total_memory() as f64 * 100.0) as u64,
    };

    // CPU
    let cpu = models::CpuStats {
        percent: sys.global_cpu_usage(),
        model: sys.cpus()[0].brand().to_string(),
        cores: sys.cpus().len(),
    };

    // Disk (Fixed)
    let disks = Disks::new_with_refreshed_list();

    // Find only the root partition
    let root_disk = disks.iter().find(|d| d.mount_point() == std::path::Path::new("/"));

    let disk = if let Some(d) = root_disk {
        let total_bytes = d.total_space();
        let available_bytes = d.available_space();
        let used_bytes = total_bytes - available_bytes;
        let mb_factor = 1024 * 1024;

        models::DiskStats {
            total: total_bytes / mb_factor,
            used: used_bytes / mb_factor,
            available: available_bytes / mb_factor,
            percent: (used_bytes as f64 / total_bytes as f64 * 100.0) as u64,
        }
    } else {
        // Fallback if "/" isn't found (prevents crash)
        models::DiskStats { total: 0, used: 0, available: 0, percent: 0 }
    };

    // Uptime
    let seconds = System::uptime();
    let uptime = models::UptimeStats {
        seconds,
        days: seconds / 86400,
        hours: (seconds % 86400) / 3600,
        minutes: (seconds % 3600) / 60,
    };

    // Network (bytes, same as original)
    let networks = Networks::new_with_refreshed_list();
    let network: HashMap<String, models::NetworkStats> = networks
        .iter()
        .map(|(name, data): (&String, &sysinfo::NetworkData)| {
            (
                name.clone(),
                models::NetworkStats {
                    rx: data.total_received(),
                    tx: data.total_transmitted(),
                },
            )
        })
        .collect();

    // Load average
    let load = System::load_average();
    let load_avg = models::LoadAvgStats {
        one: load.one,
        five: load.five,
        fifteen: load.fifteen,
    };

    // Temperature
    let components = Components::new_with_refreshed_list();
    let temperature: f32 = components
        .iter()
        .next()
        .and_then(|c: &sysinfo::Component| c.temperature())
        .unwrap_or(0.0f32);

    // Services — check a list of known services via systemctl
    let service_names = vec![
        "syncthing",
        "caddy",
        "sshd",
        "cloudflare-dyndns.timer",
        "cloudflare-dyndns",
        "docker",
    ];
    let mut services: HashMap<String, String> = HashMap::new();
    for name in service_names {
        let output = Command::new("systemctl")
            .args(["is-active", name])
            .output()
            .await;
        let status = match output {
            Ok(out) => String::from_utf8_lossy(&out.stdout).trim().to_string(),
            Err(_) => "unknown".to_string(),
        };
        services.insert(name.to_string(), status);
    }

    Json(models::SystemStats {
        timestamp: chrono::Utc::now().to_rfc3339(),
        memory,
        cpu,
        disk,
        uptime,
        network,
        load_avg,
        temperature,
        services,
    })
}
