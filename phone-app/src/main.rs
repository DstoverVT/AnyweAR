use anyhow::{Context, Result};
use anywear_phone_app::{ChunkIngestRequest, PhoneAppClient};
use chrono::{DateTime, Duration, Utc};
use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};
use url::Url;

#[derive(Debug, Parser)]
#[command(name = "anywear-phone-app")]
#[command(about = "CLI phone-side relay for the AnyweAR server")]
struct Cli {
    #[arg(long, default_value = "http://127.0.0.1:3000/")]
    server: Url,
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Health,
    SendChunk {
        #[arg(long)]
        seq: u64,
        #[arg(long)]
        start_ts: DateTime<Utc>,
        #[arg(long)]
        storage_uri: String,
    },
    SendDir {
        #[arg(long)]
        dir: PathBuf,
        #[arg(long, default_value_t = 0)]
        start_seq: u64,
        #[arg(long)]
        start_ts: DateTime<Utc>,
    },
    Events,
}

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
            storage_uri,
        } => {
            let request = build_chunk_request(seq, start_ts, storage_uri);
            let response = client.send_chunk(request).await?;
            print_chunk_result(&response);
        }
        Command::SendDir {
            dir,
            start_seq,
            start_ts,
        } => {
            let uris = collect_video_paths(&dir)?;
            for (index, uri) in uris.into_iter().enumerate() {
                let seq = start_seq + index as u64;
                let chunk_start = start_ts + Duration::seconds((index as i64) * 30);
                let request = build_chunk_request(seq, chunk_start, uri);
                let response = client.send_chunk(request).await?;
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

fn collect_video_paths(dir: &Path) -> Result<Vec<String>> {
    let mut entries = std::fs::read_dir(dir)
        .with_context(|| format!("failed to read directory {}", dir.display()))?
        .collect::<std::io::Result<Vec<_>>>()
        .with_context(|| format!("failed to enumerate directory {}", dir.display()))?;

    entries.sort_by_key(|entry| entry.path());

    let mut uris = Vec::new();
    for entry in entries {
        let path = entry.path();
        if !path.is_file() || !is_video_file(&path) {
            continue;
        }

        let absolute = path
            .canonicalize()
            .with_context(|| format!("failed to canonicalize {}", path.display()))?;
        let uri = Url::from_file_path(&absolute)
            .map_err(|_| anyhow::anyhow!("failed to build file URI for {}", absolute.display()))?;
        uris.push(uri.to_string());
    }

    Ok(uris)
}

fn is_video_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|ext| ext.to_str()),
        Some("mp4" | "mov" | "m4v" | "avi" | "mkv")
    )
}

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
