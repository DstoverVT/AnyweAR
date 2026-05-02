//! Shared API and processing domain types for the AnyweAR server.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Request body sent by a phone relay when a video chunk is ready to ingest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkIngestRequest {
    /// Monotonic sequence number assigned by the recording device.
    pub chunk_seq: u64,
    /// Timestamp for the first frame covered by the chunk.
    pub start_ts: DateTime<Utc>,
    /// Timestamp immediately after the final frame covered by the chunk.
    pub end_ts: DateTime<Utc>,
    /// URI where the server or model can retrieve the chunk media.
    pub storage_uri: String,
}

/// Response returned after a chunk has been accepted by the server.
#[derive(Debug, Clone, Serialize)]
pub struct ChunkIngestResponse {
    /// Persisted representation of the accepted chunk.
    pub chunk: VideoChunk,
    /// Completed four-chunk analysis window, if this ingest finished one.
    pub completed_window: Option<AnalysisWindow>,
    /// Whether background processing was scheduled for the completed window.
    pub processing_started: bool,
}

/// Server-side record for a single uploaded or externally stored video chunk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoChunk {
    /// Unique identifier assigned by the server.
    pub id: Uuid,
    /// Device-provided sequence number used to form fixed-size windows.
    pub chunk_seq: u64,
    /// Timestamp for the beginning of the chunk.
    pub start_ts: DateTime<Utc>,
    /// Timestamp for the end of the chunk.
    pub end_ts: DateTime<Utc>,
    /// URI pointing at the chunk media.
    pub storage_uri: String,
}

/// Four consecutive video chunks grouped for model analysis.
#[derive(Debug, Clone, Serialize)]
pub struct AnalysisWindow {
    /// Unique identifier for this analysis window.
    pub id: Uuid,
    /// First chunk sequence number included in the window.
    pub source_chunk_start: u64,
    /// Last chunk sequence number included in the window.
    pub source_chunk_end: u64,
    /// Timestamp for the beginning of the window.
    pub start_ts: DateTime<Utc>,
    /// Timestamp for the end of the window.
    pub end_ts: DateTime<Utc>,
    /// Ordered media URIs sent to the general event extractor.
    pub chunk_storage_uris: Vec<String>,
}

/// General memory event produced by the first-pass video analysis model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralEvent {
    /// Event start timestamp.
    pub start_ts: DateTime<Utc>,
    /// Event end timestamp.
    pub end_ts: DateTime<Utc>,
    /// Broad category used for journaling and downstream routing.
    pub event_type: String,
    /// Relative notability score from 0.0 to 1.0.
    pub importance: f32,
    /// Specific location when known, or a placeholder such as `unknown`.
    pub location: String,
    /// Short activity label or phrase.
    pub activity: String,
    /// Salient objects visible or relevant during the event.
    pub objects: Vec<String>,
    /// Human-readable event summary.
    pub description: String,
    /// Compact text optimized for retrieval and search.
    pub search_text: String,
    /// Model confidence score from 0.0 to 1.0.
    pub confidence: f32,
    /// Suggested follow-up extractors for more specialized analysis.
    pub trigger_candidates: Vec<String>,
}

/// Stored event with source-window metadata attached.
#[derive(Debug, Clone, Serialize)]
pub struct StoredEvent {
    /// Unique identifier assigned when the event is persisted.
    pub id: Uuid,
    /// First source chunk sequence number contributing to the event.
    pub source_chunk_start: u64,
    /// Last source chunk sequence number contributing to the event.
    pub source_chunk_end: u64,
    /// Event start timestamp.
    pub start_ts: DateTime<Utc>,
    /// Event end timestamp.
    pub end_ts: DateTime<Utc>,
    /// Broad category used for journaling and downstream routing.
    pub event_type: String,
    /// Relative notability score from 0.0 to 1.0.
    pub importance: f32,
    /// Specific location when known, or a placeholder such as `unknown`.
    pub location: String,
    /// Short activity label or phrase.
    pub activity: String,
    /// Salient objects visible or relevant during the event.
    pub objects: Vec<String>,
    /// Human-readable event summary.
    pub description: String,
    /// Compact text optimized for retrieval and search.
    pub search_text: String,
    /// Model confidence score from 0.0 to 1.0.
    pub confidence: f32,
    /// Suggested follow-up extractors for more specialized analysis.
    pub trigger_candidates: Vec<String>,
}

/// Result returned after processing a completed analysis window.
#[derive(Debug, Clone, Serialize)]
pub struct ProcessWindowResponse {
    /// Window that was processed.
    pub window: AnalysisWindow,
    /// Events inserted or merged as a result of processing.
    pub stored_events: Vec<StoredEvent>,
}
