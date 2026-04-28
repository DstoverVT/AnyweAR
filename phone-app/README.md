# AnyweAR Phone App Crate

This crate is a small Rust client that acts as the phone-side relay for the current server scaffold.

It can:

- check server health
- send individual `30-second` chunk metadata records
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
  --storage-uri file:///tmp/chunk-0.mp4
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

- This crate currently sends chunk metadata only. It does not upload raw video bytes to the server yet.
- `send-dir` converts local file paths into `file://` URIs before sending them.
- Once the fourth chunk in a group arrives, the server automatically starts background processing.
