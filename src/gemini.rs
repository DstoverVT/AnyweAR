//! Gemini-backed and mock implementations of general event extraction.

use crate::config::Config;
use crate::domain::{AnalysisWindow, GeneralEvent};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use mime_guess::MimeGuess;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::time::{sleep, Duration};
use url::Url;

/// Base URL for the Gemini REST API.
const GEMINI_API_BASE: &str = "https://generativelanguage.googleapis.com/v1beta";
/// Poll interval while Gemini processes uploaded video files.
const FILE_POLL_INTERVAL: Duration = Duration::from_secs(5);
/// Maximum number of polling attempts before a file is treated as stuck.
const FILE_POLL_ATTEMPTS: usize = 60;

/// Extracts general memory events from an analysis window.
#[async_trait]
pub trait GeneralEventExtractor: Send + Sync {
    /// Produces zero or more general events for a completed video window.
    ///
    /// # Arguments
    ///
    /// * `window` - Completed analysis window to inspect.
    ///
    /// # Returns
    ///
    /// A vector of extracted [`GeneralEvent`] values.
    ///
    /// # Errors
    ///
    /// Returns an error when extraction cannot complete.
    async fn extract(&self, window: &AnalysisWindow) -> Result<Vec<GeneralEvent>>;
}

/// Builds the configured event extractor, falling back to a mock extractor without an API key.
///
/// # Arguments
///
/// * `config` - Runtime configuration containing Gemini credentials and model selection.
///
/// # Returns
///
/// A boxed [`GeneralEventExtractor`] implementation.
pub fn build_extractor(config: &Config) -> Box<dyn GeneralEventExtractor> {
    match &config.gemini_api_key {
        Some(api_key) => Box::new(GeminiExtractor::new(
            Client::new(),
            api_key.clone(),
            config.gemini_model.clone(),
        )),
        None => Box::new(MockExtractor),
    }
}

/// Deterministic extractor used for local development without Gemini credentials.
pub struct MockExtractor;

#[async_trait]
impl GeneralEventExtractor for MockExtractor {
    /// Returns one low-confidence routine event spanning the whole window.
    ///
    /// # Arguments
    ///
    /// * `window` - Completed analysis window to cover with the mock event.
    ///
    /// # Returns
    ///
    /// A single routine [`GeneralEvent`] spanning the full window.
    ///
    /// # Errors
    ///
    /// The mock implementation currently does not return errors.
    async fn extract(&self, window: &AnalysisWindow) -> Result<Vec<GeneralEvent>> {
        Ok(vec![GeneralEvent {
            start_ts: window.start_ts,
            end_ts: window.end_ts,
            event_type: "routine".to_string(),
            importance: 0.25,
            location: "unknown".to_string(),
            activity: "background_capture".to_string(),
            objects: Vec::new(),
            description: "General background activity during the captured interval.".to_string(),
            search_text: "background activity routine captured interval".to_string(),
            confidence: 0.35,
            trigger_candidates: Vec::new(),
        }])
    }
}

/// Gemini-backed extractor for first-pass general memory events.
pub struct GeminiExtractor {
    /// HTTP client used for Gemini API calls.
    client: Client,
    /// API key sent in the Gemini request header.
    api_key: String,
    /// Gemini model identifier.
    model: String,
}

impl GeminiExtractor {
    /// Creates a Gemini extractor with an injected HTTP client and model configuration.
    ///
    /// # Arguments
    ///
    /// * `client` - HTTP client used to call Gemini.
    /// * `api_key` - Gemini API key sent with requests.
    /// * `model` - Gemini model name used in the request endpoint.
    ///
    /// # Returns
    ///
    /// A configured [`GeminiExtractor`].
    pub fn new(client: Client, api_key: String, model: String) -> Self {
        Self {
            client,
            api_key,
            model,
        }
    }

    /// Builds the model-specific `generateContent` endpoint URL.
    ///
    /// # Returns
    ///
    /// Fully qualified Gemini `generateContent` endpoint URL for the configured model.
    fn endpoint(&self) -> String {
        format!("{GEMINI_API_BASE}/models/{}:generateContent", self.model)
    }

    /// Builds the Gemini Files API upload start endpoint.
    fn files_upload_endpoint(&self) -> String {
        format!("{GEMINI_API_BASE}/upload/v1beta/files")
    }

    /// Builds the Gemini Files API get endpoint for a file name such as `files/abc123`.
    fn file_get_endpoint(&self, file_name: &str) -> String {
        format!("{GEMINI_API_BASE}/{file_name}")
    }

    /// Builds the text instruction sent alongside the video file references.
    ///
    /// # Arguments
    ///
    /// * `window` - Analysis window whose timing and chunk range should be included in the prompt.
    ///
    /// # Returns
    ///
    /// Prompt text instructing Gemini to return structured general events.
    fn prompt(window: &AnalysisWindow) -> String {
        format!(
            "You are extracting general memory events from a wearable first-person video window.\n\
Return a JSON array of event objects that cover the two-minute interval.\n\
Use broad event_type values suitable for journaling and routing specific follow-up models.\n\
Include only salient objects. Keep description concise and search_text compact.\n\
The window spans from {} to {} and covers chunk sequence numbers {} through {}.",
            window.start_ts.to_rfc3339(),
            window.end_ts.to_rfc3339(),
            window.source_chunk_start,
            window.source_chunk_end
        )
    }

    /// Returns the JSON schema Gemini should use for structured event output.
    ///
    /// # Returns
    ///
    /// A JSON Schema value describing the expected array of general event objects.
    fn schema() -> Value {
        json!({
            "type": "array",
            "items": {
                "type": "object",
                "properties": {
                    "start_ts": {
                        "type": "string",
                        "format": "date-time",
                        "description": "Event start timestamp in RFC 3339 format."
                    },
                    "end_ts": {
                        "type": "string",
                        "format": "date-time",
                        "description": "Event end timestamp in RFC 3339 format."
                    },
                    "event_type": {
                        "type": "string",
                        "description": "Broad event class such as meal, drink, routine, work, travel, household_task, object_placement, rest, or unknown."
                    },
                    "importance": {
                        "type": "number",
                        "minimum": 0.0,
                        "maximum": 1.0,
                        "description": "How notable the event is for summaries and surfacing."
                    },
                    "location": {
                        "type": "string",
                        "description": "Specific location if known, otherwise unknown."
                    },
                    "activity": {
                        "type": "string",
                        "description": "Specific activity phrase such as eating, preparing_food, working_at_desk, or walking."
                    },
                    "objects": {
                        "type": "array",
                        "description": "A short list of salient objects relevant to memory recall.",
                        "items": {
                            "type": "string"
                        },
                        "maxItems": 8
                    },
                    "description": {
                        "type": "string",
                        "description": "Short human-readable description of the event."
                    },
                    "search_text": {
                        "type": "string",
                        "description": "Compact retrieval-oriented phrase for semantic and full-text search."
                    },
                    "confidence": {
                        "type": "number",
                        "minimum": 0.0,
                        "maximum": 1.0,
                        "description": "Model confidence in the extracted event."
                    },
                    "trigger_candidates": {
                        "type": "array",
                        "description": "Specific-layer follow-up checks that should run next, such as meal_detail or alcohol_check.",
                        "items": {
                            "type": "string"
                        }
                    }
                },
                "required": [
                    "start_ts",
                    "end_ts",
                    "event_type",
                    "importance",
                    "location",
                    "activity",
                    "objects",
                    "description",
                    "search_text",
                    "confidence",
                    "trigger_candidates"
                ],
                "additionalProperties": false
            }
        })
    }

    /// Resolves a stored chunk URI into a Gemini file reference.
    ///
    /// Local `file://` URIs are uploaded to Gemini Files API on demand and polled until active.
    /// Other URIs are assumed to already be Gemini-readable references.
    async fn resolve_file_data(&self, uri: &str) -> Result<FileData> {
        let parsed = Url::parse(uri).with_context(|| format!("invalid chunk storage URI {uri}"))?;
        if parsed.scheme() == "file" {
            return self.upload_local_file_uri(&parsed).await;
        }

        Ok(FileData {
            mime_type: infer_mime_type(uri).to_string(),
            file_uri: uri.to_string(),
        })
    }

    async fn upload_local_file_uri(&self, uri: &Url) -> Result<FileData> {
        let path = uri
            .to_file_path()
            .map_err(|_| anyhow!("failed to convert local file URI into a path: {uri}"))?;
        let bytes = tokio::fs::read(&path)
            .await
            .with_context(|| format!("failed to read local chunk {}", path.display()))?;
        let mime_type = infer_mime_type(path.to_string_lossy().as_ref()).to_string();
        let display_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("chunk.mp4")
            .to_string();

        let uploaded = self
            .upload_file_bytes(bytes, mime_type.clone(), display_name)
            .await?;
        let active = self.wait_for_file_active(uploaded).await?;

        Ok(FileData {
            mime_type: active.mime_type.unwrap_or(mime_type),
            file_uri: active.uri,
        })
    }

    async fn upload_file_bytes(
        &self,
        bytes: Vec<u8>,
        mime_type: String,
        display_name: String,
    ) -> Result<GeminiFileMetadata> {
        let start_response = self
            .client
            .post(self.files_upload_endpoint())
            .header("x-goog-api-key", &self.api_key)
            .header("X-Goog-Upload-Protocol", "resumable")
            .header("X-Goog-Upload-Command", "start")
            .header(
                "X-Goog-Upload-Header-Content-Length",
                bytes.len().to_string(),
            )
            .header("X-Goog-Upload-Header-Content-Type", &mime_type)
            .json(&json!({
                "file": {
                    "display_name": display_name,
                }
            }))
            .send()
            .await
            .context("failed to start Gemini file upload")?
            .error_for_status()
            .context("Gemini file upload start returned an error status")?;

        let upload_url = start_response
            .headers()
            .get("x-goog-upload-url")
            .ok_or_else(|| {
                anyhow!("Gemini file upload response did not include x-goog-upload-url")
            })?
            .to_str()
            .context("Gemini upload URL header was not valid UTF-8")?
            .to_string();

        let finalize_response = self
            .client
            .post(upload_url)
            .header("Content-Length", bytes.len().to_string())
            .header("X-Goog-Upload-Offset", "0")
            .header("X-Goog-Upload-Command", "upload, finalize")
            .header("Content-Type", "application/octet-stream")
            .body(bytes)
            .send()
            .await
            .context("failed to upload bytes to Gemini Files API")?
            .error_for_status()
            .context("Gemini file upload finalize returned an error status")?;

        let file: GeminiFileEnvelope = finalize_response
            .json()
            .await
            .context("failed to deserialize Gemini uploaded file metadata")?;

        Ok(file.file)
    }

    async fn wait_for_file_active(
        &self,
        mut file: GeminiFileMetadata,
    ) -> Result<GeminiFileMetadata> {
        for _ in 0..FILE_POLL_ATTEMPTS {
            if file.state_is_active() {
                return Ok(file);
            }

            if file.state_is_failed() {
                return Err(anyhow!(
                    "Gemini file {} failed processing in state {:?}",
                    file.name,
                    file.state
                ));
            }

            sleep(FILE_POLL_INTERVAL).await;
            file = self.fetch_file(&file.name).await?;
        }

        Err(anyhow!(
            "Gemini file {} did not become ACTIVE within the polling window",
            file.name
        ))
    }

    async fn fetch_file(&self, file_name: &str) -> Result<GeminiFileMetadata> {
        let response = self
            .client
            .get(self.file_get_endpoint(file_name))
            .header("x-goog-api-key", &self.api_key)
            .send()
            .await
            .with_context(|| format!("failed to fetch Gemini file status for {file_name}"))?
            .error_for_status()
            .with_context(|| format!("Gemini file status returned an error for {file_name}"))?;

        let payload: GeminiFileEnvelope = response
            .json()
            .await
            .with_context(|| format!("failed to deserialize Gemini file status for {file_name}"))?;
        Ok(payload.file)
    }
}

/// Request envelope for Gemini `generateContent`.
#[derive(Debug, Serialize)]
struct GenerateContentRequest {
    /// Ordered content entries sent to the model.
    contents: Vec<Content>,
    /// Response format and schema controls.
    #[serde(rename = "generationConfig")]
    generation_config: GenerationConfig,
}

/// Gemini content container made of text and file parts.
#[derive(Debug, Serialize)]
struct Content {
    /// Parts that make up a single content message.
    parts: Vec<Part>,
}

/// Individual Gemini content part.
#[derive(Debug, Serialize)]
#[serde(untagged)]
enum Part {
    /// Plain text prompt part.
    Text {
        /// Text sent directly to the model.
        text: String,
    },
    /// File reference part pointing Gemini at externally stored media.
    FileData {
        /// Gemini file metadata for this part.
        #[serde(rename = "file_data")]
        file_data: FileData,
    },
}

/// Gemini file reference metadata.
#[derive(Debug, Serialize)]
struct FileData {
    /// MIME type of the referenced file.
    mime_type: String,
    /// URI for the referenced file.
    file_uri: String,
}

/// Minimal Gemini file metadata envelope.
#[derive(Debug, Deserialize)]
struct GeminiFileEnvelope {
    /// Uploaded or fetched file metadata.
    file: GeminiFileMetadata,
}

/// Gemini file metadata used to poll until the file can be consumed by generateContent.
#[derive(Debug, Deserialize)]
struct GeminiFileMetadata {
    /// Resource name such as `files/abc123`.
    name: String,
    /// File URI to pass into `file_data`.
    uri: String,
    /// Optional MIME type reported by Gemini.
    #[serde(rename = "mimeType")]
    mime_type: Option<String>,
    /// Processing state.
    state: Option<GeminiFileState>,
}

impl GeminiFileMetadata {
    fn state_is_active(&self) -> bool {
        self.state.as_ref().and_then(GeminiFileState::name) == Some("ACTIVE")
    }

    fn state_is_failed(&self) -> bool {
        matches!(
            self.state.as_ref().and_then(GeminiFileState::name),
            Some("FAILED" | "CANCELLED")
        )
    }
}

/// Gemini file state representation varies between plain strings and nested objects.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum GeminiFileState {
    /// State string such as `ACTIVE`.
    Plain(String),
    /// Nested form such as `{ "name": "PROCESSING" }`.
    Named { name: String },
}

impl GeminiFileState {
    fn name(&self) -> Option<&str> {
        match self {
            Self::Plain(value) => Some(value.as_str()),
            Self::Named { name } => Some(name.as_str()),
        }
    }
}

/// Gemini generation configuration for structured JSON output.
#[derive(Debug, Serialize)]
struct GenerationConfig {
    /// Desired response MIME type.
    #[serde(rename = "responseMimeType")]
    response_mime_type: String,
    /// JSON schema Gemini should satisfy.
    #[serde(rename = "responseJsonSchema")]
    response_json_schema: Value,
}

/// Gemini `generateContent` response envelope.
#[derive(Debug, Deserialize)]
struct GenerateContentResponse {
    /// Candidate model responses, if any were produced.
    candidates: Option<Vec<Candidate>>,
}

/// One candidate response from Gemini.
#[derive(Debug, Deserialize)]
struct Candidate {
    /// Candidate content payload.
    content: ContentResponse,
}

/// Content returned by Gemini.
#[derive(Debug, Deserialize)]
struct ContentResponse {
    /// Parts returned in the candidate content.
    parts: Vec<PartResponse>,
}

/// One response part returned by Gemini.
#[derive(Debug, Deserialize)]
struct PartResponse {
    /// JSON text emitted by the model.
    text: Option<String>,
}

/// Direct deserialization shape for Gemini-generated general events.
#[derive(Debug, Deserialize)]
struct RawGeneralEvent {
    /// Event start timestamp.
    start_ts: DateTime<Utc>,
    /// Event end timestamp.
    end_ts: DateTime<Utc>,
    /// Broad event category.
    event_type: String,
    /// Relative notability score from 0.0 to 1.0.
    importance: f32,
    /// Location label produced by the model.
    location: String,
    /// Activity phrase produced by the model.
    activity: String,
    /// Salient objects named by the model.
    objects: Vec<String>,
    /// Human-readable event summary.
    description: String,
    /// Retrieval-oriented search text.
    search_text: String,
    /// Model confidence score from 0.0 to 1.0.
    confidence: f32,
    /// Suggested specialized follow-up extractors.
    trigger_candidates: Vec<String>,
}

#[async_trait]
impl GeneralEventExtractor for GeminiExtractor {
    /// Calls Gemini with the window media and converts its JSON output into events.
    ///
    /// # Arguments
    ///
    /// * `window` - Completed analysis window containing media URIs to send to Gemini.
    ///
    /// # Returns
    ///
    /// A vector of [`GeneralEvent`] values parsed from Gemini's JSON response.
    ///
    /// # Errors
    ///
    /// Returns an error when the window has no media URIs, the Gemini request fails, the response
    /// envelope cannot be decoded, or the model output is not valid event JSON.
    async fn extract(&self, window: &AnalysisWindow) -> Result<Vec<GeneralEvent>> {
        if window.chunk_storage_uris.is_empty() {
            return Err(anyhow!("analysis window does not contain any chunk URIs"));
        }

        let mut parts = Vec::with_capacity(window.chunk_storage_uris.len() + 1);
        parts.push(Part::Text {
            text: Self::prompt(window),
        });
        for chunk_uri in &window.chunk_storage_uris {
            parts.push(Part::FileData {
                file_data: self.resolve_file_data(chunk_uri).await?,
            });
        }

        let request = GenerateContentRequest {
            contents: vec![Content { parts }],
            generation_config: GenerationConfig {
                response_mime_type: "application/json".to_string(),
                response_json_schema: Self::schema(),
            },
        };

        let response = self
            .client
            .post(self.endpoint())
            .header("x-goog-api-key", &self.api_key)
            .json(&request)
            .send()
            .await
            .context("failed to call Gemini generateContent")?
            .error_for_status()
            .context("Gemini generateContent returned an error status")?;

        let payload: GenerateContentResponse = response
            .json()
            .await
            .context("failed to deserialize Gemini response envelope")?;

        let raw_json = payload
            .candidates
            .unwrap_or_default()
            .into_iter()
            .flat_map(|candidate| candidate.content.parts)
            .find_map(|part| part.text)
            .ok_or_else(|| anyhow!("Gemini response did not include JSON text"))?;

        let events: Vec<RawGeneralEvent> =
            serde_json::from_str(&raw_json).context("failed to parse Gemini JSON output")?;

        Ok(events
            .into_iter()
            .map(|event| GeneralEvent {
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
            })
            .collect())
    }
}

fn infer_mime_type(uri_or_path: &str) -> &'static str {
    MimeGuess::from_path(uri_or_path)
        .first_raw()
        .unwrap_or("video/mp4")
}
