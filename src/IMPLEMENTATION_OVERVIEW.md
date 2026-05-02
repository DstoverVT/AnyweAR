# Implementation Overview

This document summarizes the current Rust server scaffold for the AnyweAR general-memory pipeline.

The repository now also includes a separate `phone-app` crate that acts as a phone-side relay client. This document focuses on the server implementation in `src/`.

## Current Goal

The server currently supports the first implementation slice:

- ingest `30-second` chunk metadata
- ingest uploaded chunk media files from the phone relay
- group `4` consecutive chunks into a `2-minute` analysis window
- run a general event extractor on that window
- store normalized event records in memory
- expose simple HTTP endpoints for ingestion, processing, and inspection

This is a foundation for the eventual full system, not the finished production architecture.

## High-Level Flow

1. A client sends chunk metadata to `POST /chunks` or uploads a local chunk file to `POST /chunks/upload`.
2. Uploaded files are persisted locally on the server and represented internally as `file://` URIs.
3. The server stores the chunk in the in-memory repository.
4. When chunk sequences `N` through `N+3` are all present, the server creates one completed analysis window.
5. The server automatically starts background processing for that completed window.
6. The server runs the configured general event extractor for that window.
7. When Gemini sees a local `file://` chunk URI, it uploads that file to the Gemini Files API and polls until the file is `ACTIVE`.
8. Extracted events are converted into stored event records.
9. Adjacent compatible events are merged.
10. Stored events are visible through `GET /events`.

## Module Breakdown

### `main.rs`

Application entrypoint.

- loads configuration from environment
- constructs the extractor
- builds shared application state
- starts the Axum HTTP server

### `config.rs`

Environment-backed runtime configuration.

Current config values:

- `BIND_ADDR`
- `GEMINI_API_KEY`
- `GEMINI_MODEL`

### `domain.rs`

Shared domain types used across HTTP, services, and extraction.

Important types:

- `ChunkIngestRequest`
- `ChunkIngestResponse`
- `VideoChunk`
- `AnalysisWindow`
- `GeneralEvent`
- `StoredEvent`
- `ProcessWindowResponse`

The general event format reflects the design we settled on:

- `event_type`
- `importance`
- `location`
- `activity`
- `objects`
- `description`
- `search_text`
- `confidence`
- `trigger_candidates`

### `http.rs`

Axum route definitions and HTTP handlers.

Current endpoints:

- `GET /health`
- `POST /chunks`
- `POST /chunks/upload`
- `GET /events`

The HTTP layer is intentionally thin and forwards most behavior to the service layer.

### `service.rs`

Application orchestration layer.

Responsibilities:

- validate ingestion input
- persist uploaded chunk media to a local temporary directory
- insert chunks into the repository
- detect when a 4-chunk window is complete
- spawn background processing for completed windows
- run the general extractor
- convert extracted events into stored events
- persist merged event results

`AppState` currently contains:

- an in-memory repository protected by `tokio::sync::Mutex`
- a polymorphic general event extractor

### `repository.rs`

In-memory storage implementation.

Responsibilities:

- store chunks by sequence number
- create completed windows when four consecutive chunks exist
- prevent duplicate window creation for the same starting sequence
- store and merge event records

Merge behavior currently requires:

- adjacent source chunk ranges
- same `event_type`
- same `location`
- same `activity`

When merging:

- `source_chunk_end` is extended
- `end_ts` is extended
- `importance` and `confidence` keep the max value
- `objects` and `trigger_candidates` are unioned
- `description` and `search_text` are replaced by the newer event

### `gemini.rs`

General event extraction interface and implementations.

#### `GeneralEventExtractor`

Trait that abstracts the extraction backend:

```rust
#[async_trait]
pub trait GeneralEventExtractor: Send + Sync {
    async fn extract(&self, window: &AnalysisWindow) -> Result<Vec<GeneralEvent>>;
}
```

#### `MockExtractor`

Used when `GEMINI_API_KEY` is not set.

- returns a placeholder `routine` event
- keeps the pipeline runnable during development

#### `GeminiExtractor`

Current responsibilities:

- build a Gemini `generateContent` request
- upload local `file://` chunks to the Gemini Files API when needed
- poll uploaded video files until Gemini marks them `ACTIVE`
- include prompt text describing the 2-minute window
- include one `file_data` part per chunk URI
- request JSON output constrained by a response schema
- deserialize returned JSON into `GeneralEvent`

## Current API Shape

### `POST /chunks`

Example request body:

```json
{
  "chunk_seq": 0,
  "start_ts": "2026-04-27T12:00:00Z",
  "end_ts": "2026-04-27T12:00:30Z",
  "storage_uri": "gs://example/chunk-0.mp4"
}
```

Behavior:

- stores the chunk
- computes the relevant 4-chunk window start
- returns a completed window once all 4 chunks are available
- starts background processing automatically when a window completes

### `POST /chunks/upload`

Multipart form fields:

- `seq`
- `start_ts`
- `end_ts`
- `file`

Behavior:

- reads uploaded chunk bytes from the phone relay
- stores the media in the server temp directory
- converts the stored path into a `file://` URI
- ingests the chunk using the normal repository path
- starts background processing automatically when a window completes

### `GET /events`

Behavior:

- returns all current stored events from memory

## Important Design Decisions Already Implemented

- `30-second` chunks are the ingestion unit
- `2-minute` windows are the general-layer inference unit
- no overlapping inference windows
- all stored records are treated as events
- `event_type` is broad and stable
- `search_text` is included for future retrieval
- `trigger_candidates` supports routing to the future specific layer

## What Is Not Implemented Yet

The current scaffold intentionally does not include:

- Postgres persistence
- object storage integration
- durable background job processing
- automatic specific-layer execution
- user feedback storage
- query answering API
- daily journal generation
- authentication or multi-user support

## Recommended Next Steps

1. Replace the in-memory repository with Postgres-backed storage.
2. Add a job queue so completed windows are processed automatically.
3. Persist specific-layer jobs keyed off `trigger_candidates`.
4. Add a retrieval/query layer on top of stored events.
5. Add daily journal generation from ordered stored events.
6. Add cleanup and reuse strategies for local uploads and Gemini file resources.

## Practical Notes

- The current scaffold is optimized for validating the pipeline shape, not for durability.
- The Gemini integration is structured so it can later be swapped or extended without rewriting the service layer.
- The extractor trait is the main seam for future experimentation with hosted APIs or local models.
