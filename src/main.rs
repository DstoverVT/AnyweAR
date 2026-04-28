mod config;
mod domain;
mod error;
mod gemini;
mod http;
mod repository;
mod service;

use crate::config::Config;
use crate::gemini::build_extractor;
use crate::service::AppState;
use anyhow::Context;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = Config::from_env();
    let extractor = Arc::from(build_extractor(&config));
    let state = Arc::new(AppState::new(extractor));
    let app = http::router(state);

    let listener = TcpListener::bind(&config.bind_addr)
        .await
        .with_context(|| format!("failed to bind {}", config.bind_addr))?;

    tracing::info!("listening on {}", config.bind_addr);

    axum::serve(listener, app).await.context("server error")
}
