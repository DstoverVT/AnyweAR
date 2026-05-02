//! Client library for sending phone-side video chunk metadata to the AnyweAR server.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use url::Url;
use uuid::Uuid;

/// HTTP client for the phone-side relay API calls.
#[derive(Debug, Clone)]
pub struct PhoneAppClient {
    /// Base URL for the AnyweAR server.
    base_url: Url,
    /// Shared HTTP client used for requests.
    http: Client,
}

impl PhoneAppClient {
    /// Creates a phone app client for the given server base URL.
    ///
    /// # Arguments
    ///
    /// * `base_url` - Base URL for the AnyweAR server, usually ending in a slash.
    ///
    /// # Returns
    ///
    /// A [`PhoneAppClient`] configured with a fresh HTTP client.
    pub fn new(base_url: Url) -> Self {
        Self {
            base_url,
            http: Client::new(),
        }
    }

    /// Calls the server health endpoint and returns its plain-text response.
    ///
    /// # Returns
    ///
    /// The plain-text response body from the health endpoint.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails, the server returns a non-success status, or the
    /// response body cannot be read.
    pub async fn health(&self) -> Result<String> {
        let response = self
            .http
            .get(self.endpoint("health")?)
            .send()
            .await
            .context("failed to call health endpoint")?
            .error_for_status()
            .context("health endpoint returned an error status")?;

        response
            .text()
            .await
            .context("failed to read health response")
    }

    /// Sends one chunk ingest request to the server.
    ///
    /// # Arguments
    ///
    /// * `request` - Chunk ingest payload to POST to the server.
    ///
    /// # Returns
    ///
    /// The server's [`ChunkIngestResponse`].
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails, the server returns a non-success status, or the JSON
    /// response cannot be decoded.
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

    /// Fetches all stored events from the server.
    ///
    /// # Returns
    ///
    /// A vector of stored events returned by the server.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails, the server returns a non-success status, or the JSON
    /// response cannot be decoded.
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

    /// Resolves an endpoint path against the configured server base URL.
    ///
    /// # Arguments
    ///
    /// * `path` - Relative endpoint path such as `health`, `chunks`, or `events`.
    ///
    /// # Returns
    ///
    /// A fully resolved endpoint URL.
    ///
    /// # Errors
    ///
    /// Returns an error if the path cannot be joined to the configured base URL.
    fn endpoint(&self, path: &str) -> Result<Url> {
        self.base_url
            .join(path)
            .with_context(|| format!("failed to build endpoint URL for path {path}"))
    }
}

/// Request body sent when the phone relay publishes a video chunk.
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

/// Response returned after the server accepts a chunk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkIngestResponse {
    /// Persisted representation of the accepted chunk.
    pub chunk: VideoChunk,
    /// Completed four-chunk analysis window, if this ingest finished one.
    pub completed_window: Option<AnalysisWindow>,
    /// Whether background processing was scheduled for the completed window.
    pub processing_started: bool,
}

/// Client-side copy of the server's video chunk record.
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

/// Client-side copy of a completed analysis window.
#[derive(Debug, Clone, Serialize, Deserialize)]
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

/// Client-side view of an event stored by the server.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
