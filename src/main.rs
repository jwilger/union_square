use anyhow::Result;
use tracing::{info, instrument};
use union_square::Application;

#[tokio::main]
#[instrument]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    info!("Starting Union Square application");

    let app = Application::new().await?;
    app.run().await?;

    Ok(())
}