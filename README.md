# AnyweAR Workspace

Rust workspace for the AnyweAR ingestion and general-memory pipeline.

## Crates

- `anywear-server`
  - the server-side ingestion, batching, and general-memory extraction scaffold
- `anywear-phone-app`
  - a phone-side Rust client crate and CLI for sending chunk metadata to the server and reading events

## What exists

- `POST /chunks`
  - stores incoming `30s` chunk metadata
  - returns a completed `2-minute` analysis window once `4` consecutive chunks are present
  - automatically starts background processing for that completed window
- `GET /events`
  - lists normalized stored events
- adjacent matching events are merged in memory across back-to-back windows

## Run

```bash
cargo run -p anywear-server
```

Optional environment variables:

```bash
export BIND_ADDR=0.0.0.0:3000
export GEMINI_API_KEY=your_api_key
export GEMINI_MODEL=gemini-3-flash-preview
```

Run the phone-side CLI:

```bash
cargo run -p anywear-phone-app -- --help
```

## Example flow

Start the server in one terminal:

```bash
cargo run -p anywear-server
```

Send chunk records from the phone-side CLI in another terminal:

```bash
cargo run -p anywear-phone-app -- send-chunk \
  --seq 0 \
  --start-ts 2026-04-27T12:00:00Z \
  --storage-uri file:///tmp/chunk-0.mp4
```

Or create four chunk records directly:

```bash
curl -X POST http://localhost:3000/chunks \
  -H 'content-type: application/json' \
  -d '{
    "chunk_seq": 0,
    "start_ts": "2026-04-27T12:00:00Z",
    "end_ts": "2026-04-27T12:00:30Z",
    "storage_uri": "gs://example/chunk-0.mp4"
  }'
```

Repeat for `chunk_seq` `1`, `2`, and `3`. The fourth response includes `completed_window.id`.

List stored events:

```bash
curl http://localhost:3000/events
```

Or from the phone-side CLI:

```bash
cargo run -p anywear-phone-app -- events
```

## Next steps

- replace the in-memory repository with Postgres
- upload the combined `2-minute` window to object storage or Gemini Files API before extraction
- replace in-process background tasks with a durable background job runner
- persist specific-layer jobs linked to general event ids
