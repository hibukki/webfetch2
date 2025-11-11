use anyhow::Result;
use rmcp::{ServiceExt, transport::stdio};
use tracing_subscriber::EnvFilter;
use webfetch2::WebFetch;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing with stderr output
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(tracing::Level::DEBUG.into()))
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    tracing::info!("Starting WebFetch2 MCP server");

    // Create service and serve over stdio
    let service = WebFetch::new().serve(stdio()).await.inspect_err(|e| {
        tracing::error!("serving error: {:?}", e);
    })?;

    // Wait for shutdown
    service.waiting().await?;
    Ok(())
}
