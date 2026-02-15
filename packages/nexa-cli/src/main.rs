use anyhow::{Context, Result, bail};
use cargo_metadata::MetadataCommand;
use clap::{Parser, Subcommand, ValueEnum};
use log::{error, info, warn};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
mod build_wasm;
mod dev_server;

use notify::{RecursiveMode, Watcher};
use std::sync::mpsc::channel;

#[derive(Parser)]
#[command(name = "nexa")]
#[command(version = "0.1.0")]
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
        /// Project template
        #[arg(long, value_enum, default_value_t = Template::Web)]
        template: Template,
    },
    /// Build the project
    Build {
        /// Build for release
        #[arg(long)]
        release: bool,
        /// Target platform (detects automatically if omitted)
        #[arg(long)]
        target: Option<String>,
    },
    /// Run the development server
    Dev {
        /// Watch for changes
        #[arg(long, default_value_t = true)]
        watch: bool,
    },
    /// Serve the static assets
    Serve {
        /// Port to listen on
        #[arg(long, default_value = "8080")]
        port: u16,
        /// Directory to serve
        #[arg(long, default_value = "dist")]
        dir: String,
    },
    /// Scan workspace and show metadata
    Scan,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
enum Template {
    Web,
    Desktop,
    Mobile,
    Fullstack,
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    let cli = Cli::parse();

    match cli.command {
        Commands::New { name, template } => create_new_project(&name, template)?,
        Commands::Build { release, target } => build_project(release, target)?,
        Commands::Dev { watch } => run_dev(watch).await?,
        Commands::Serve { port, dir } => serve_dir(port, &dir).await?,
        Commands::Scan => {
            let root = scan_workspace()?;
            println!("Nexa Workspace detected at: {}", root.display());
        }
    }

    Ok(())
}

fn create_new_project(name: &str, template: Template) -> Result<()> {
    info!("Creating new Nexa {:?} project: {}", template, name);

    // Create directory
    fs::create_dir_all(name).context("Failed to create project directory")?;
    let project_path = Path::new(name);

    // Initial cargo init
    Command::new("cargo")
        .args(&["init", name, "--lib"])
        .status()
        .context("Failed to run cargo init")?;

    // Scaffold based on template
    match template {
        Template::Web => scaffold_web(project_path)?,
        Template::Desktop => scaffold_desktop(project_path)?,
        Template::Mobile => scaffold_mobile(project_path)?,
        Template::Fullstack => scaffold_fullstack(project_path)?,
    }

    info!("Project {} created successfully!", name);
    Ok(())
}

fn scaffold_web(path: &Path) -> Result<()> {
    let cargo_toml = r#"[package]
name = "my-nexa-app"
version = "0.1.0"
edition = "2024"

[dependencies]
nexa-core = { git = "https://github.com/nexa-rs/nexa" }
nexa-web = { git = "https://github.com/nexa-rs/nexa" }
"#;
    fs::write(path.join("Cargo.toml"), cargo_toml)?;
    fs::write(
        path.join("index.html"),
        "<html><body><div id=\"main\"></div><script type=\"module\" src=\"/src/main.rs\"></script></body></html>",
    )?;
    Ok(())
}

fn scaffold_desktop(_path: &Path) -> Result<()> {
    // Scaffold desktop specifics
    Ok(())
}

fn scaffold_mobile(_path: &Path) -> Result<()> {
    // Scaffold mobile specifics (Android/iOS dirs)
    Ok(())
}

fn scaffold_fullstack(_path: &Path) -> Result<()> {
    // Scaffold fullstack (SSR + API)
    Ok(())
}

fn build_project(release: bool, target: Option<String>) -> Result<()> {
    let metadata = MetadataCommand::new()
        .exec()
        .context("Failed to read cargo metadata")?;
    let _root = &metadata.workspace_root;

    let resolved_target = target.unwrap_or_else(|| {
        // Detect target from dependencies
        "web".to_string() // default for now
    });

    info!("Building for target: {}", resolved_target);

    let mut args = vec!["build"];
    if release {
        args.push("--release");
    }

    if resolved_target == "web" {
        // Run wasm-pack or nexa-bundler
        info!("Running WASM bundling hooks...");
    }

    let status = Command::new("cargo")
        .args(&args)
        .status()
        .context("Failed to run cargo build")?;

    if !status.success() {
        bail!("Build failed with status {}", status);
    }

    Ok(())
}

async fn run_dev(watch: bool) -> Result<()> {
    // Detect if this is a web project (has index.html)
    if Path::new("index.html").exists() {
        info!("Nexa Web Project detected.");

        // 1. Check Requirements
        build_wasm::check_requirements()?;

        // 2. Initial Build
        let metadata = MetadataCommand::new()
            .exec()
            .context("Failed to read cargo metadata")?;
        let package = metadata
            .packages
            .iter()
            .find(|p| p.manifest_path.parent().unwrap() == std::env::current_dir().unwrap())
            .context("Could not find package for current directory")?;

        let project_name = &package.name;

        info!("Building {}...", project_name);
        build_wasm::build_project(false)?;
        build_wasm::run_bindgen(false, project_name)?;
        build_wasm::generate_dist(project_name)?;

        info!("Build successful!");

        // 3. Start Server
        let port = 8080;
        tokio::spawn(async move {
            if let Err(e) = dev_server::serve(port).await {
                error!("Server error: {}", e);
            }
        });

        if !watch {
            tokio::signal::ctrl_c().await?;
            return Ok(());
        }

        // 4. Watch Loop
        let (tx, rx) = channel();
        let mut watcher = notify::recommended_watcher(tx)?;
        watcher.watch(Path::new("src"), RecursiveMode::Recursive)?;
        watcher.watch(Path::new("index.html"), RecursiveMode::NonRecursive)?;

        info!("Watching for changes...");

        for res in rx {
            match res {
                Ok(_) => {
                    info!("Change detected. Rebuilding...");
                    match build_rebuild(project_name) {
                        Ok(_) => info!("Rebuild successful!"),
                        Err(e) => error!("Rebuild failed: {}", e),
                    }
                }
                Err(e) => error!("Watch error: {}", e),
            }
        }

        return Ok(());
    }

    // Legacy/Native handling
    info!("Starting dev server (watch={})...", watch);
    if watch {
        // Check for cargo-watch
        let status = Command::new("cargo").args(&["watch", "-x", "run"]).status();

        if status.is_err() || !status.unwrap().success() {
            warn!("cargo-watch not found, falling back to simple run");
            Command::new("cargo").arg("run").status()?;
        }
    } else {
        Command::new("cargo").arg("run").status()?;
    }
    Ok(())
}

fn build_rebuild(project_name: &str) -> Result<()> {
    // Only rebuild, do not crash server
    build_wasm::build_project(false)?;
    build_wasm::run_bindgen(false, project_name)?;
    build_wasm::generate_dist(project_name)?;
    Ok(())
}

async fn serve_dir(port: u16, dir: &str) -> Result<()> {
    use std::net::SocketAddr;
    use tower_http::services::ServeDir;

    info!("Serving '{}' on http://localhost:{}", dir, port);

    let app = axum::Router::new().nest_service("/", ServeDir::new(dir));
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let listener = tokio::net::TcpListener::bind(addr).await?;

    axum::serve(listener, app).await?;
    Ok(())
}

fn scan_workspace() -> Result<PathBuf> {
    let metadata = MetadataCommand::new()
        .exec()
        .context("Failed to read cargo metadata")?;

    Ok(metadata.workspace_root.into())
}
