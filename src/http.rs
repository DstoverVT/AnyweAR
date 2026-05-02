//! HTTP routes for chunk ingest, event listing, and health checks.

use crate::domain::{ChunkIngestRequest, ChunkIngestResponse, StoredEvent};
use crate::error::AppError;
use crate::service::{ingest_chunk, list_events, spawn_window_processing, AppState};
use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use std::sync::Arc;

/// Builds the HTTP router for the AnyweAR API.
///
/// # Arguments
///
/// * `state` - Shared application state attached to each route.
///
/// # Returns
///
/// An Axum [`Router`] with health, chunk ingest, and event routes registered.
pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/chunks", post(create_chunk))
        .route("/events", get(events))
        .with_state(state)
}

/// Returns a lightweight readiness response.
///
/// # Returns
///
/// The static health response body.
async fn health() -> &'static str {
    "ok"
}

/// Accepts a chunk ingest request and starts window processing when a window completes.
///
/// # Arguments
///
/// * `state` - Shared application state extracted from the router.
/// * `payload` - JSON chunk ingest payload sent by the phone relay.
///
/// # Returns
///
/// A JSON chunk ingest response describing the stored chunk and any completed window.
///
/// # Errors
///
/// Returns [`AppError`] when validation, storage, or processing setup fails.
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

/// Returns all currently stored general events.
///
/// # Arguments
///
/// * `state` - Shared application state extracted from the router.
///
/// # Returns
///
/// A JSON array containing the current stored event snapshot.
async fn events(State(state): State<Arc<AppState>>) -> Json<Vec<StoredEvent>> {
    Json(list_events(&state).await)
}
