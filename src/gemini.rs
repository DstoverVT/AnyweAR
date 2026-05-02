//! Gemini-backed and mock implementations of general event extraction.

use crate::config::Config;
use crate::domain::{AnalysisWindow, GeneralEvent};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// Base URL for the Gemini REST API.
const GEMINI_API_BASE: &str = "https://generativelanguage.googleapis.com/v1beta";

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
        parts.extend(
            window
                .chunk_storage_uris
                .iter()
                .cloned()
                .map(|file_uri| Part::FileData {
                    file_data: FileData {
                        mime_type: "video/mp4".to_string(),
                        file_uri,
                    },
                }),
        );

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
