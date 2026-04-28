use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use url::Url;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct PhoneAppClient {
    base_url: Url,
    http: Client,
}

impl PhoneAppClient {
    pub fn new(base_url: Url) -> Self {
        Self {
            base_url,
            http: Client::new(),
        }
    }

    pub async fn health(&self) -> Result<String> {
        let response = self
            .http
            .get(self.endpoint("health")?)
            .send()
            .await
            .context("failed to call health endpoint")?
            .error_for_status()
            .context("health endpoint returned an error status")?;

        response.text().await.context("failed to read health response")
    }

    pub async fn send_chunk(&self, request: ChunkIngestRequest) -> Result<ChunkIngestResponse> {
        let response = self
            .http
            .post(self.endpoint("chunks")?)
            .json(&request)
            .send()
            .await
            .context("failed to send chunk to server")?
            .error_for_status()
            .context("chunk endpoint returned an error status")?;

        response
            .json()
            .await
            .context("failed to deserialize chunk ingest response")
    }

    pub async fn list_events(&self) -> Result<Vec<StoredEvent>> {
        let response = self
            .http
            .get(self.endpoint("events")?)
            .send()
            .await
            .context("failed to fetch stored events")?
            .error_for_status()
            .context("events endpoint returned an error status")?;

        response
            .json()
            .await
            .context("failed to deserialize stored events")
    }

    fn endpoint(&self, path: &str) -> Result<Url> {
        self.base_url
            .join(path)
            .with_context(|| format!("failed to build endpoint URL for path {path}"))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkIngestRequest {
    pub chunk_seq: u64,
    pub start_ts: DateTime<Utc>,
    pub end_ts: DateTime<Utc>,
    pub storage_uri: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisWindow {
    pub id: Uuid,
    pub source_chunk_start: u64,
    pub source_chunk_end: u64,
    pub start_ts: DateTime<Utc>,
    pub end_ts: DateTime<Utc>,
    pub chunk_storage_uris: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
