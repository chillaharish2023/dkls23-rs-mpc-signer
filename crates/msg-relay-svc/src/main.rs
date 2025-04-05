//! Message Relay Service
//!
//! HTTP/WebSocket service for routing MPC messages between parties.

use anyhow::Result;
use axum::{
    extract::{Path, State, WebSocketUpgrade},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use clap::Parser;
use msg_relay::{MessageId, MessageStore, StoredMessage};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing::{info, Level};

/// Message relay service CLI arguments
#[derive(Parser, Debug)]
#[command(name = "msg-relay-svc")]
#[command(about = "Message relay service for MPC communication")]
struct Args {
    /// Listen address
    #[arg(short, long, default_value = "0.0.0.0:8080")]
    listen: String,

    /// Peer relay URLs
    #[arg(short, long)]
    peer: Vec<String>,

    /// Message TTL in seconds
    #[arg(long, default_value = "3600")]
    ttl: i64,
}

/// Application state
struct AppState {
    store: MessageStore,
    peers: Vec<String>,
}

/// Request to post a message
#[derive(Debug, Serialize, Deserialize)]
struct PostMessageRequest {
    session_id: String,
    round: u32,
    from: Option<usize>,
    to: Option<usize>,
    tag: String,
    payload: String, // base64 encoded
}

/// Request to get a message
#[derive(Debug, Serialize, Deserialize)]
struct GetMessageRequest {
    session_id: String,
    round: u32,
    from: Option<usize>,
    to: Option<usize>,
    tag: String,
}

/// Message response
#[derive(Debug, Serialize, Deserialize)]
struct MessageResponse {
    found: bool,
    payload: Option<String>, // base64 encoded
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(Level::INFO.into()),
        )
        .init();

    let args = Args::parse();

    info!(
        listen = %args.listen,
        peers = ?args.peer,
        ttl = args.ttl,
        "Starting message relay service"
    );

    let state = Arc::new(AppState {
        store: MessageStore::new(args.ttl),
        peers: args.peer,
    });

    // Spawn cleanup task
    let cleanup_store = state.store.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
        loop {
            interval.tick().await;
            cleanup_store.cleanup();
        }
    });

    let app = Router::new()
        .route("/health", get(health))
        .route("/v1/msg", post(post_message))
        .route("/v1/msg", get(get_message))
        .route("/v1/msg/:hash", get(get_message_by_hash))
        .route("/v1/ws", get(websocket_handler))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(&args.listen).await?;
    info!(address = %args.listen, "Listening");

    axum::serve(listener, app).await?;

    Ok(())
}

/// Health check endpoint
async fn health() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "ok",
        "service": "msg-relay-svc",
        "version": env!("CARGO_PKG_VERSION")
    }))
}

/// Post a message to the relay
async fn post_message(
    State(state): State<Arc<AppState>>,
    Json(req): Json<PostMessageRequest>,
) -> impl IntoResponse {
    let id = MessageId::new(&req.session_id, req.round, req.from, req.to, &req.tag);

    let payload = match b64::decode(&req.payload) {
        Ok(p) => p,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": format!("Invalid base64: {}", e) })),
            );
        }
    };

    if let Err(e) = state.store.put(id.clone(), payload) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        );
    }

    info!(
        session_id = %req.session_id,
        round = req.round,
        from = ?req.from,
        to = ?req.to,
        "Message stored"
    );

    (
        StatusCode::OK,
        Json(serde_json::json!({ "hash": id.hash() })),
    )
}

/// Get a message from the relay
async fn get_message(
    State(state): State<Arc<AppState>>,
    Json(req): Json<GetMessageRequest>,
) -> impl IntoResponse {
    let id = MessageId::new(&req.session_id, req.round, req.from, req.to, &req.tag);

    match state.store.get(&id) {
        Ok(msg) => Json(MessageResponse {
            found: true,
            payload: Some(b64::encode(&msg.payload)),
        }),
        Err(_) => Json(MessageResponse {
            found: false,
            payload: None,
        }),
    }
}

/// Get a message by hash
async fn get_message_by_hash(
    State(state): State<Arc<AppState>>,
    Path(hash): Path<String>,
) -> impl IntoResponse {
    // Search for message with matching hash
    // This is a simplified implementation
    Json(MessageResponse {
        found: false,
        payload: None,
    })
}

/// WebSocket handler for real-time messaging
async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_websocket(socket, state))
}

async fn handle_websocket(
    socket: axum::extract::ws::WebSocket,
    state: Arc<AppState>,
) {
    use axum::extract::ws::Message;
    use futures_util::{SinkExt, StreamExt};

    let (mut sender, mut receiver) = socket.split();

    while let Some(msg) = receiver.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                // Echo for now - real implementation would handle MPC messages
                let _ = sender.send(Message::Text(text)).await;
            }
            Ok(Message::Close(_)) => break,
            _ => {}
        }
    }
}

mod b64 {
    use base64::{engine::general_purpose::STANDARD, Engine};

    pub fn encode(data: &[u8]) -> String {
        STANDARD.encode(data)
    }

    pub fn decode(s: &str) -> Result<Vec<u8>, base64::DecodeError> {
        STANDARD.decode(s)
    }
}
