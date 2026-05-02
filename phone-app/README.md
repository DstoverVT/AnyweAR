# AnyweAR Phone App Crate

This crate is a small Rust client that acts as the phone-side relay for the current server scaffold.

It can:

- check server health
- upload individual `30-second` chunk video files
- send a whole directory of local video files as sequential chunks
- fetch stored events from the server

## Run

```bash
cargo run -p anywear-phone-app -- --help
```

## Examples

Health check:

```bash
cargo run -p anywear-phone-app -- health
```

Send one chunk:

```bash
cargo run -p anywear-phone-app -- send-chunk \
  --seq 0 \
  --start-ts 2026-04-27T12:00:00Z \
  --path /tmp/chunk-0.mp4
```

Send one chunk by metadata only when you already have a Gemini-readable URI:

```bash
cargo run -p anywear-phone-app -- send-chunk \
  --seq 0 \
  --start-ts 2026-04-27T12:00:00Z \
  --storage-uri gs://example/chunk-0.mp4
```

Send a directory of chunk files:

```bash
cargo run -p anywear-phone-app -- send-dir \
  --dir /tmp/chunks \
  --start-seq 0 \
  --start-ts 2026-04-27T12:00:00Z
```

List stored events:

```bash
cargo run -p anywear-phone-app -- events
```

## Notes

- `send-chunk --path` and `send-dir` upload raw video bytes to `POST /chunks/upload`.
- The server stores uploaded media locally and, when Gemini is enabled, registers those files with
  the Gemini Files API during extraction.
- `send-chunk --storage-uri` is still available for pre-hosted media URIs.
- Once the fourth chunk in a group arrives, the server automatically starts background processing.
