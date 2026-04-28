use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkIngestRequest {
    pub chunk_seq: u64,
    pub start_ts: DateTime<Utc>,
    pub end_ts: DateTime<Utc>,
    pub storage_uri: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChunkIngestResponse {
    pub chunk: VideoChunk,
    pub completed_window: Option<AnalysisWindow>,
    pub processing_started: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoChunk {
    pub id: Uuid,
    pub chunk_seq: u64,
    pub start_ts: DateTime<Utc>,
    pub end_ts: DateTime<Utc>,
    pub storage_uri: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AnalysisWindow {
    pub id: Uuid,
    pub source_chunk_start: u64,
    pub source_chunk_end: u64,
    pub start_ts: DateTime<Utc>,
    pub end_ts: DateTime<Utc>,
    pub chunk_storage_uris: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralEvent {
    pub start_ts: DateTime<Utc>,
    pub end_ts: DateTime<Utc>,
    pub event_type: String,
    pub importance: f32,
    pub location: String,
    pub activity: String,
    pub objects: Vec<String>,
    pub description: String,
    pub search_text: String,
    pub confidence: f32,
    pub trigger_candidates: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct StoredEvent {
    pub id: Uuid,
    pub source_chunk_start: u64,
    pub source_chunk_end: u64,
    pub start_ts: DateTime<Utc>,
    pub end_ts: DateTime<Utc>,
    pub event_type: String,
    pub importance: f32,
    pub location: String,
    pub activity: String,
    pub objects: Vec<String>,
    pub description: String,
    pub search_text: String,
    pub confidence: f32,
    pub trigger_candidates: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProcessWindowResponse {
    pub window: AnalysisWindow,
    pub stored_events: Vec<StoredEvent>,
}
