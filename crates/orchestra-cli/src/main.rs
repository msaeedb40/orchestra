//! CLI binary entry point (`orchestra`).

use clap::Parser;
use orchestra_api::OrchestraClient;
use orchestra_cli::cli::Cli;
use orchestra_cli::commands::handle_command;
use tonic::transport::Endpoint;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    if cli.verbose {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::DEBUG)
            .init();
    }

    let channel = match Endpoint::from_shared(cli.daemon_url.clone())?
        .connect()
        .await
    {
        Ok(c) => c,
        Err(e) => {
            eprintln!(
                "Error: Failed to connect to Orchestra daemon at {}: {}",
                cli.daemon_url, e
            );
            eprintln!("Is 'orchestrad' running? Start it with: orchestrad");
            std::process::exit(1);
        }
    };

    let client = OrchestraClient::new(channel);

    if let Err(e) = handle_command(client, cli.command, cli.output).await {
        eprintln!("Command error: {e}");
        std::process::exit(1);
    }

    Ok(())
}
