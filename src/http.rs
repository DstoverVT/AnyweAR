use crate::domain::{ChunkIngestRequest, ChunkIngestResponse, StoredEvent};
use crate::error::AppError;
use crate::service::{ingest_chunk, list_events, spawn_window_processing, AppState};
use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use std::sync::Arc;

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/chunks", post(create_chunk))
        .route("/events", get(events))
        .with_state(state)
}

async fn health() -> &'static str {
    "ok"
}

async fn create_chunk(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<ChunkIngestRequest>,
) -> Result<Json<ChunkIngestResponse>, AppError> {
    let response = ingest_chunk(&state, payload).await?;
    if let Some(window) = &response.completed_window {
        spawn_window_processing(Arc::clone(&state), window.id);
    }
    Ok(Json(response))
}

async fn events(State(state): State<Arc<AppState>>) -> Json<Vec<StoredEvent>> {
    Json(list_events(&state).await)
}
