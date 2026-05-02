//! HTTP routes for chunk ingest, event listing, and health checks.

use crate::domain::{ChunkIngestRequest, ChunkIngestResponse, StoredEvent};
use crate::error::AppError;
use crate::service::{
    ingest_chunk, ingest_uploaded_chunk, list_events, spawn_window_processing, AppState,
};
use axum::{
    extract::{Multipart, State},
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
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
        .route("/chunks/upload", post(upload_chunk))
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

/// Accepts one uploaded chunk media file, stores it locally on the server, and ingests its
/// metadata so later processing can upload it to Gemini.
async fn upload_chunk(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> Result<Json<ChunkIngestResponse>, AppError> {
    let mut seq = None;
    let mut start_ts = None;
    let mut end_ts = None;
    let mut file_name = None;
    let mut mime_type = None;
    let mut file_bytes = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|error| AppError::BadRequest(format!("failed to read multipart field: {error}")))?
    {
        match field.name() {
            Some("seq") => {
                let value = field.text().await.map_err(|error| {
                    AppError::BadRequest(format!("failed to read seq field: {error}"))
                })?;
                seq = Some(parse_seq(&value)?);
            }
            Some("start_ts") => {
                let value = field.text().await.map_err(|error| {
                    AppError::BadRequest(format!("failed to read start_ts field: {error}"))
                })?;
                start_ts = Some(parse_ts("start_ts", &value)?);
            }
            Some("end_ts") => {
                let value = field.text().await.map_err(|error| {
                    AppError::BadRequest(format!("failed to read end_ts field: {error}"))
                })?;
                end_ts = Some(parse_ts("end_ts", &value)?);
            }
            Some("file") => {
                file_name = field.file_name().map(str::to_string);
                mime_type = field.content_type().map(str::to_string);
                file_bytes = Some(field.bytes().await.map_err(|error| {
                    AppError::BadRequest(format!("failed to read uploaded file bytes: {error}"))
                })?);
            }
            _ => {
                let _ = field.bytes().await;
            }
        }
    }

    let request = ChunkIngestRequest {
        chunk_seq: seq.ok_or_else(|| AppError::BadRequest("missing seq field".to_string()))?,
        start_ts: start_ts
            .ok_or_else(|| AppError::BadRequest("missing start_ts field".to_string()))?,
        end_ts: end_ts.ok_or_else(|| AppError::BadRequest("missing end_ts field".to_string()))?,
        storage_uri: String::new(),
    };

    let file_bytes = file_bytes
        .ok_or_else(|| AppError::BadRequest("missing file field".to_string()))?
        .to_vec();

    let response = ingest_uploaded_chunk(&state, request, file_bytes, file_name, mime_type).await?;
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

fn parse_seq(value: &str) -> Result<u64, AppError> {
    value
        .parse::<u64>()
        .map_err(|error| AppError::BadRequest(format!("invalid seq field: {error}")))
}

fn parse_ts(field_name: &str, value: &str) -> Result<DateTime<Utc>, AppError> {
    DateTime::parse_from_rfc3339(value)
        .map(|timestamp| timestamp.with_timezone(&Utc))
        .map_err(|error| AppError::BadRequest(format!("invalid {field_name} field: {error}")))
}
