use clap::{Parser, Subcommand};
use anyhow::{Result, Context};
use std::process::Command;
use std::path::{Path, PathBuf};
use cargo_metadata::MetadataCommand;

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
        /// Build for release
        #[arg(long)]
        release: bool,
    },
    /// Check the project for errors
    Check,
    /// Run the development server
    Dev,
    /// Serve the static assets
    Serve {
        /// Port to listen on
        #[arg(long, default_value = "8080")]
        port: u16,
        /// Directory to serve
        #[arg(long, default_value = "dist")]
        dir: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::New { name } => create_new_project(&name)?,
        Commands::Build { release } => build_project(release)?,
        Commands::Check => check_project()?,
        Commands::Dev => run_dev()?,
        Commands::Serve { port, dir } => serve_dir(port, &dir).await?,
    }

    Ok(())
}

fn create_new_project(name: &str) -> Result<()> {
    println!("Creating new Nexa project: {}", name);
    // Basic cargo new wrapper for now
    Command::new("cargo")
        .args(&["new", name])
        .status()
        .context("Failed to run cargo new")?;
        
    // In a real CLI, we would modify Cargo.toml to add nexa dependencies
    // and scaffold a basic App structure.
    let project_path = Path::new(name);
    let _cargo_toml_path = project_path.join("Cargo.toml");
    
    // Append nexa dep if we could (mocking this step as implied by "scaffold")
    // println!("Adding nexa dependency..."); 
    
    Ok(())
}

fn build_project(release: bool) -> Result<()> {
    println!("Building project...");
    let mut args = vec!["build"];
    if release {
        args.push("--release");
    }
    
    let status = Command::new("cargo")
        .args(&args)
        .status()
        .context("Failed to run cargo build")?;
        
    if !status.success() {
        anyhow::bail!("Build failed");
    }
    
    Ok(())
}

fn check_project() -> Result<()> {
    println!("Checking project...");
    let status = Command::new("cargo")
        .arg("check")
        .status()
        .context("Failed to run cargo check")?;
        
    if !status.success() {
        anyhow::bail!("Check failed");
    }
    
    Ok(())
}

fn run_dev() -> Result<()> {
    println!("Starting dev server (wrapper around cargo run)...");
    // This assumes the user project is a binary that runs a server (e.g. nexa-fullstack)
    // or a desktop app. 
    // For web, this would need trunk or wasm-pack.
    // We'll assume a generic cargo run for now.
    
    let status = Command::new("cargo")
        .arg("run")
        .status()
        .context("Failed to run cargo run")?;

    if !status.success() {
        anyhow::bail!("Dev run failed");
    }

    Ok(())
}

async fn serve_dir(port: u16, dir: &str) -> Result<()> {
    use axum::Router;
    use tower_http::services::ServeDir;
    use std::net::SocketAddr;

    println!("Serving directory '{}' on http://localhost:{}", dir, port);

    let app = Router::new().nest_service("/", ServeDir::new(dir));
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    
    axum::serve(listener, app).await?;
    
    Ok(())
}

// Workspace scanning helper (unused in this minimal impl but requested)
// We'll add a helper that can be expanded later.
#[allow(dead_code)]
fn scan_workspace() -> Result<PathBuf> {
    let metadata = MetadataCommand::new()
        .exec()
        .context("Failed to read cargo metadata")?;
        
    Ok(metadata.workspace_root.into())
}
