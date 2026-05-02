//! Application services that coordinate repository state and event extraction.

use crate::domain::{
    AnalysisWindow, ChunkIngestRequest, ChunkIngestResponse, GeneralEvent, ProcessWindowResponse,
    StoredEvent, VideoChunk,
};
use crate::error::AppError;
use crate::gemini::GeneralEventExtractor;
use crate::repository::InMemoryRepository;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{error, info};
use uuid::Uuid;

/// Shared application state used by HTTP handlers and background tasks.
pub struct AppState {
    /// In-memory persistence for chunks, windows, and extracted events.
    pub repository: Mutex<InMemoryRepository>,
    /// General event extractor used to analyze completed windows.
    pub extractor: Arc<dyn GeneralEventExtractor>,
}

impl AppState {
    /// Creates application state with an empty repository and configured extractor.
    ///
    /// # Arguments
    ///
    /// * `extractor` - Event extractor implementation used for completed windows.
    ///
    /// # Returns
    ///
    /// A new [`AppState`] with an empty in-memory repository.
    pub fn new(extractor: Arc<dyn GeneralEventExtractor>) -> Self {
        Self {
            repository: Mutex::new(InMemoryRepository::default()),
            extractor,
        }
    }
}

/// Stores a chunk and returns a completed analysis window when four consecutive chunks are ready.
///
/// # Arguments
///
/// * `state` - Application state containing the repository to update.
/// * `request` - Chunk metadata sent by the phone relay.
///
/// # Returns
///
/// A [`ChunkIngestResponse`] containing the stored chunk and optional completed window.
///
/// # Errors
///
/// Returns [`AppError::BadRequest`] when the chunk end timestamp is not later than its start
/// timestamp.
pub async fn ingest_chunk(
    state: &AppState,
    request: ChunkIngestRequest,
) -> Result<ChunkIngestResponse, AppError> {
    if request.end_ts <= request.start_ts {
        return Err(AppError::BadRequest(
            "chunk end_ts must be later than start_ts".to_string(),
        ));
    }

    let chunk = VideoChunk {
        id: Uuid::new_v4(),
        chunk_seq: request.chunk_seq,
        start_ts: request.start_ts,
        end_ts: request.end_ts,
        storage_uri: request.storage_uri,
    };

    let completed_window = {
        let mut repo = state.repository.lock().await;
        repo.insert_chunk(chunk.clone());

        let start_seq = chunk.chunk_seq - (chunk.chunk_seq % 4);
        repo.build_completed_window(start_seq)
    };

    Ok(ChunkIngestResponse {
        chunk,
        processing_started: completed_window.is_some(),
        completed_window,
    })
}

/// Spawns background processing for an already completed analysis window.
///
/// # Arguments
///
/// * `state` - Shared application state moved into the background task.
/// * `window_id` - Identifier of the completed analysis window to process.
///
/// # Returns
///
/// This function returns immediately after spawning the task.
pub fn spawn_window_processing(state: Arc<AppState>, window_id: Uuid) {
    tokio::spawn(async move {
        match process_window(&state, window_id).await {
            Ok(result) => {
                info!(
                    window_id = %result.window.id,
                    stored_event_count = result.stored_events.len(),
                    "completed background processing for analysis window"
                );
            }
            Err(err) => {
                error!(
                    window_id = %window_id,
                    error = %err,
                    "failed background processing for analysis window"
                );
            }
        }
    });
}

/// Runs extraction for a window and persists the resulting stored events.
///
/// # Arguments
///
/// * `state` - Application state containing the repository and extractor.
/// * `window_id` - Identifier of the completed analysis window to process.
///
/// # Returns
///
/// A [`ProcessWindowResponse`] containing the processed window and stored events.
///
/// # Errors
///
/// Returns [`AppError::NotFound`] if the window ID does not exist, or [`AppError::Internal`] if
/// event extraction fails.
pub async fn process_window(
    state: &AppState,
    window_id: Uuid,
) -> Result<ProcessWindowResponse, AppError> {
    let window = {
        let repo = state.repository.lock().await;
        repo.get_window(window_id)
            .ok_or_else(|| AppError::NotFound(format!("analysis window {window_id}")))?
    };

    let extracted_events = state
        .extractor
        .extract(&window)
        .await
        .map_err(|error| AppError::Internal(error.to_string()))?;

    let stored = extracted_events
        .into_iter()
        .map(|event| to_stored_event(&window, event))
        .collect::<Vec<_>>();

    let stored_events = {
        let mut repo = state.repository.lock().await;
        repo.upsert_events(stored)
    };

    Ok(ProcessWindowResponse {
        window,
        stored_events,
    })
}

/// Returns a snapshot of all stored events.
///
/// # Arguments
///
/// * `state` - Application state containing the repository to read.
///
/// # Returns
///
/// A cloned vector of all stored events in repository order.
pub async fn list_events(state: &AppState) -> Vec<StoredEvent> {
    let repo = state.repository.lock().await;
    repo.events().to_vec()
}

/// Converts a model event into a persisted event tied to its source window.
///
/// # Arguments
///
/// * `window` - Analysis window that produced the model event.
/// * `event` - General event returned by the extractor.
///
/// # Returns
///
/// A [`StoredEvent`] with a new ID and source chunk metadata from the window.
fn to_stored_event(window: &AnalysisWindow, event: GeneralEvent) -> StoredEvent {
    StoredEvent {
        id: Uuid::new_v4(),
        source_chunk_start: window.source_chunk_start,
        source_chunk_end: window.source_chunk_end,
        start_ts: event.start_ts,
        end_ts: event.end_ts,
        event_type: event.event_type,
        importance: event.importance,
        location: event.location,
        activity: event.activity,
        objects: event.objects,
        description: event.description,
        search_text: event.search_text,
        confidence: event.confidence,
        trigger_candidates: event.trigger_candidates,
    }
}
