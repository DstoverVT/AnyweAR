use crate::config::Config;
use crate::domain::{AnalysisWindow, GeneralEvent};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

const GEMINI_API_BASE: &str = "https://generativelanguage.googleapis.com/v1beta";

#[async_trait]
pub trait GeneralEventExtractor: Send + Sync {
    async fn extract(&self, window: &AnalysisWindow) -> Result<Vec<GeneralEvent>>;
}

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

pub struct MockExtractor;

#[async_trait]
impl GeneralEventExtractor for MockExtractor {
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

pub struct GeminiExtractor {
    client: Client,
    api_key: String,
    model: String,
}

impl GeminiExtractor {
    pub fn new(client: Client, api_key: String, model: String) -> Self {
        Self {
            client,
            api_key,
            model,
        }
    }

    fn endpoint(&self) -> String {
        format!("{GEMINI_API_BASE}/models/{}:generateContent", self.model)
    }

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

#[derive(Debug, Serialize)]
struct GenerateContentRequest {
    contents: Vec<Content>,
    #[serde(rename = "generationConfig")]
    generation_config: GenerationConfig,
}

#[derive(Debug, Serialize)]
struct Content {
    parts: Vec<Part>,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
enum Part {
    Text { text: String },
    FileData {
        #[serde(rename = "file_data")]
        file_data: FileData,
    },
}

#[derive(Debug, Serialize)]
struct FileData {
    mime_type: String,
    file_uri: String,
}

#[derive(Debug, Serialize)]
struct GenerationConfig {
    #[serde(rename = "responseMimeType")]
    response_mime_type: String,
    #[serde(rename = "responseJsonSchema")]
    response_json_schema: Value,
}

#[derive(Debug, Deserialize)]
struct GenerateContentResponse {
    candidates: Option<Vec<Candidate>>,
}

#[derive(Debug, Deserialize)]
struct Candidate {
    content: ContentResponse,
}

#[derive(Debug, Deserialize)]
struct ContentResponse {
    parts: Vec<PartResponse>,
}

#[derive(Debug, Deserialize)]
struct PartResponse {
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawGeneralEvent {
    start_ts: DateTime<Utc>,
    end_ts: DateTime<Utc>,
    event_type: String,
    importance: f32,
    location: String,
    activity: String,
    objects: Vec<String>,
    description: String,
    search_text: String,
    confidence: f32,
    trigger_candidates: Vec<String>,
}

#[async_trait]
impl GeneralEventExtractor for GeminiExtractor {
    async fn extract(&self, window: &AnalysisWindow) -> Result<Vec<GeneralEvent>> {
        if window.chunk_storage_uris.is_empty() {
            return Err(anyhow!("analysis window does not contain any chunk URIs"));
        }

        let mut parts = Vec::with_capacity(window.chunk_storage_uris.len() + 1);
        parts.push(Part::Text {
            text: Self::prompt(window),
        });
        parts.extend(window.chunk_storage_uris.iter().cloned().map(|file_uri| Part::FileData {
            file_data: FileData {
                mime_type: "video/mp4".to_string(),
                file_uri,
            },
        }));

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
