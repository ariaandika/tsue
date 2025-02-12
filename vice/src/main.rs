use anyhow::Result;
use tracing_subscriber::EnvFilter;




fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    vice::run().inspect_err(|err|tracing::error!("{err:?}"))
}

