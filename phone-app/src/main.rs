//! Command-line phone relay for posting local video chunks to the AnyweAR server.

use anyhow::{Context, Result};
use anywear_phone_app::{ChunkIngestRequest, PhoneAppClient};
use chrono::{DateTime, Duration, Utc};
use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};
use url::Url;

/// Command-line arguments for the phone-side relay.
#[derive(Debug, Parser)]
#[command(name = "anywear-phone-app")]
#[command(about = "CLI phone-side relay for the AnyweAR server")]
struct Cli {
    /// Base URL for the AnyweAR server.
    #[arg(long, default_value = "http://127.0.0.1:3000/")]
    server: Url,
    /// Command to execute.
    #[command(subcommand)]
    command: Command,
}

/// Supported phone relay commands.
#[derive(Debug, Subcommand)]
enum Command {
    /// Checks whether the server is reachable.
    Health,
    /// Sends one chunk to the server, either by uploading a local file or by posting metadata.
    SendChunk {
        /// Sequence number for the chunk.
        #[arg(long)]
        seq: u64,
        /// Start timestamp for the chunk.
        #[arg(long)]
        start_ts: DateTime<Utc>,
        /// Local video file to upload to the server.
        #[arg(long, conflicts_with = "storage_uri")]
        path: Option<PathBuf>,
        /// Existing URI where the chunk media can be read.
        #[arg(long, conflicts_with = "path")]
        storage_uri: Option<String>,
    },
    /// Sends every video file in a directory as consecutive chunks.
    SendDir {
        /// Directory containing video files.
        #[arg(long)]
        dir: PathBuf,
        /// Sequence number assigned to the first video in sorted order.
        #[arg(long, default_value_t = 0)]
        start_seq: u64,
        /// Start timestamp assigned to the first video.
        #[arg(long)]
        start_ts: DateTime<Utc>,
    },
    /// Lists events currently stored by the server.
    Events,
}

/// Runs the phone-side command-line relay.
///
/// # Returns
///
/// `Ok(())` after the selected command completes successfully.
///
/// # Errors
///
/// Returns an error if argument handling, file discovery, HTTP calls, or response decoding fails.
#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let client = PhoneAppClient::new(cli.server);

    match cli.command {
        Command::Health => {
            println!("{}", client.health().await?);
        }
        Command::SendChunk {
            seq,
            start_ts,
            path,
            storage_uri,
        } => {
            let response = if let Some(path) = path {
                client
                    .upload_chunk_file(seq, start_ts, start_ts + Duration::seconds(30), &path)
                    .await?
            } else if let Some(storage_uri) = storage_uri {
                let request = build_chunk_request(seq, start_ts, storage_uri);
                client.send_chunk(request).await?
            } else {
                anyhow::bail!("send-chunk requires either --path or --storage-uri");
            };
            print_chunk_result(&response);
        }
        Command::SendDir {
            dir,
            start_seq,
            start_ts,
        } => {
            let paths = collect_video_paths(&dir)?;
            for (index, path) in paths.into_iter().enumerate() {
                let seq = start_seq + index as u64;
                let chunk_start = start_ts + Duration::seconds((index as i64) * 30);
                let response = client
                    .upload_chunk_file(seq, chunk_start, chunk_start + Duration::seconds(30), &path)
                    .await?;
                print_chunk_result(&response);
            }
        }
        Command::Events => {
            let events = client.list_events().await?;
            for event in events {
                println!(
                    "{} {}-{} {} {} {}",
                    event.id,
                    event.start_ts,
                    event.end_ts,
                    event.event_type,
                    event.location,
                    event.description
                );
            }
        }
    }

    Ok(())
}

/// Builds a thirty-second chunk ingest request from command-line inputs.
///
/// # Arguments
///
/// * `seq` - Sequence number assigned to the chunk.
/// * `start_ts` - Start timestamp for the chunk.
/// * `storage_uri` - URI where the chunk media can be read.
///
/// # Returns
///
/// A [`ChunkIngestRequest`] with `end_ts` set thirty seconds after `start_ts`.
fn build_chunk_request(
    seq: u64,
    start_ts: DateTime<Utc>,
    storage_uri: String,
) -> ChunkIngestRequest {
    ChunkIngestRequest {
        chunk_seq: seq,
        start_ts,
        end_ts: start_ts + Duration::seconds(30),
        storage_uri,
    }
}

/// Collects video file paths from a directory.
///
/// # Arguments
///
/// * `dir` - Directory to scan for supported video files.
///
/// # Returns
///
/// Sorted absolute paths for recognized video files in the directory.
///
/// # Errors
///
/// Returns an error if the directory cannot be read, an entry cannot be enumerated, a file cannot be
/// canonicalized.
fn collect_video_paths(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut entries = std::fs::read_dir(dir)
        .with_context(|| format!("failed to read directory {}", dir.display()))?
        .collect::<std::io::Result<Vec<_>>>()
        .with_context(|| format!("failed to enumerate directory {}", dir.display()))?;

    entries.sort_by_key(|entry| entry.path());

    let mut paths = Vec::new();
    for entry in entries {
        let path = entry.path();
        if !path.is_file() || !is_video_file(&path) {
            continue;
        }

        let absolute = path
            .canonicalize()
            .with_context(|| format!("failed to canonicalize {}", path.display()))?;
        paths.push(absolute);
    }

    Ok(paths)
}

/// Returns whether the path has a recognized video file extension.
///
/// # Arguments
///
/// * `path` - Path whose extension should be checked.
///
/// # Returns
///
/// `true` when the extension is one of `mp4`, `mov`, `m4v`, `avi`, or `mkv`.
fn is_video_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|ext| ext.to_str()),
        Some("mp4" | "mov" | "m4v" | "avi" | "mkv")
    )
}

/// Prints a concise summary of a chunk ingest response.
///
/// # Arguments
///
/// * `response` - Chunk ingest response to summarize on stdout.
///
/// # Returns
///
/// This function does not return a value.
fn print_chunk_result(response: &anywear_phone_app::ChunkIngestResponse) {
    println!(
        "sent chunk seq={} window_completed={} processing_started={}",
        response.chunk.chunk_seq,
        response.completed_window.is_some(),
        response.processing_started
    );

    if let Some(window) = &response.completed_window {
        println!(
            "window {} chunks {}-{} {} -> {}",
            window.id,
            window.source_chunk_start,
            window.source_chunk_end,
            window.start_ts,
            window.end_ts
        );
    }
}
