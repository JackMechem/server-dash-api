use axum::{
    body::Body,
    routing::get,
    response::Json,
    Router,
};
use serde_json::{Value, json};



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
async fn get_stats() -> Json<Value> {
    Json(json!({ "data": 67 }))
}
