use clap::{Parser, Subcommand};
use anyhow::Result;

#[derive(Parser)]
#[command(name = "nexa")]
#[command(about = "Nexa Framework CLI", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new Nexa project
    New {
        /// Project name
        name: String,
    },
    /// Build the project
    Build {
        /// Platform to build for (web, desktop, mobile)
        #[arg(long, default_value = "web")]
        platform: String,
    },
    /// Serve the project for development
    Dev {
        /// Platform to serve
        #[arg(long, default_value = "web")]
        platform: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();

    match &cli.command {
        Commands::New { name } => {
            println!("Creating new Nexa project: {}", name);
            // Minimal implementation: Create directory and Cargo.toml
            std::fs::create_dir_all(name)?;
            let cargo_toml = format!(
r#"[package]
name = "{}"
version = "0.1.0"
edition = "2024"

[dependencies]
nexa-core = "*""#, name);
            std::fs::write(format!("{}/Cargo.toml", name), cargo_toml)?;
            println!("Project created successfully.");
        }
        Commands::Build { platform } => {
            println!("Building for platform: {}", platform);
            let status = std::process::Command::new("cargo")
                .arg("build")
                .status()?;
            if !status.success() {
                anyhow::bail!("Build failed");
            }
        }
        Commands::Dev { platform } => {
            println!("Starting dev server for: {}", platform);
            // Minimal implementation: wrappers around cargo run
            let status = std::process::Command::new("cargo")
                .arg("run")
                .status()?;
             if !status.success() {
                anyhow::bail!("Dev server failed");
            }
        }
    }

    Ok(())
}
