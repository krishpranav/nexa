use anyhow::{Context, Result, bail};
use cargo_metadata::MetadataCommand;
use log::{error, info, warn};
use std::fs;
use std::path::Path;
use std::process::Command;

pub fn check_requirements() -> Result<()> {
    // 1. Check WASM target
    let output = Command::new("rustup")
        .args(&["target", "list", "--installed"])
        .output()
        .context("Failed to check installed targets")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    if !stdout.contains("wasm32-unknown-unknown") {
        info!("Target 'wasm32-unknown-unknown' not found. Installing...");
        let status = Command::new("rustup")
            .args(&["target", "add", "wasm32-unknown-unknown"])
            .status()
            .context("Failed to install wasm32 target")?;

        if !status.success() {
            bail!("Failed to install wasm32-unknown-unknown target");
        }
    }

    // 2. Check wasm-bindgen-cli
    if Command::new("wasm-bindgen")
        .arg("--version")
        .output()
        .is_err()
    {
        info!("wasm-bindgen-cli not found. Installing...");
        let status = Command::new("cargo")
            .args(&["install", "wasm-bindgen-cli"])
            .status()
            .context("Failed to install wasm-bindgen-cli")?;

        if !status.success() {
            bail!("Failed to install wasm-bindgen-cli");
        }
    }

    Ok(())
}

pub fn build_project(release: bool) -> Result<()> {
    info!("Compiling to WASM...");

    // Check if rustup is available to enforce stable toolchain
    let use_rustup = Command::new("rustup").arg("--version").output().is_ok();

    let mut command_name = "cargo".to_string();
    let args = vec!["build", "--target", "wasm32-unknown-unknown"];
    let mut rustc_path: Option<String> = None;

    // If rustup exists, use it to bypass potential Homebrew/System rust mismatches
    if use_rustup {
        // Resolve the ABSOLUTE path to the stable cargo binary
        // This avoids issues where `rustup run` fails to override Homebrew's PATH
        if let Ok(output) = Command::new("rustup")
            .args(&["which", "cargo", "--toolchain", "stable"])
            .output()
        {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path.is_empty() {
                    info!("Resolved stable cargo: {}", path);
                    command_name = path.clone();

                    // Derive rustc path (sibling to cargo)
                    let cargo_path = std::path::PathBuf::from(&path);
                    if let Some(parent) = cargo_path.parent() {
                        let rc = parent.join("rustc");
                        if rc.exists() {
                            rustc_path = Some(rc.to_string_lossy().to_string());
                            info!("Resolved stable rustc: {}", rc.display());
                        }
                    }
                }
            }
        }
    }

    let mut final_args = args.clone();
    if release {
        final_args.push("--release");
    }

    // Try build loop (max 2 attempts)
    for attempt in 1..=2 {
        let mut cmd = Command::new(&command_name);
        cmd.args(&final_args);

        if let Some(rc) = &rustc_path {
            cmd.env("RUSTC", rc);
        }

        if use_rustup {
            // CRITICAL: Prevent environment leakage from the current process
            // If nexa-cli was run via `cargo run` or `rustup run`, these vars might force
            // the inner command to use the wrong toolchain or flags.
            cmd.env_remove("RUSTUP_TOOLCHAIN");
            cmd.env_remove("RUSTFLAGS");
            cmd.env_remove("CARGO_ENCODED_RUSTFLAGS");
            cmd.env_remove("CARGO_TARGET_DIR"); // Don't share target dir with host build

            // AGGRESSIVE: Clear home dirs to force rustup to use its own defaults
            cmd.env_remove("RUSTUP_HOME");
            cmd.env_remove("CARGO_HOME");
        } else {
            cmd.args(&args);
        }

        let output = cmd
            .output() // Capture output to check for errors
            .context("Failed to run cargo build")?;

        if output.status.success() {
            // Print warnings if any
            let stderr = String::from_utf8_lossy(&output.stderr);
            if !stderr.trim().is_empty() {
                eprintln!("{}", stderr);
            }
            return Ok(());
        }

        let stderr = String::from_utf8_lossy(&output.stderr);
        eprintln!("{}", stderr); // Show user the error

        // Check for specific partial-install error or missing target error
        if (stderr.contains("E0463") || stderr.contains("target may not be installed"))
            && attempt == 1
        {
            warn!("Detected missing or sync-failed target. Attempting to repair...");

            // Force install/update target
            let status = Command::new("rustup")
                .args(&["target", "add", "wasm32-unknown-unknown"])
                .status()
                .context("Failed to run rustup target add")?;

            if status.success() {
                info!("Target repaired. Retrying build...");
                continue;
            } else {
                error!("Failed to repair target via rustup.");
            }
        }

        bail!("Cargo build failed");
    }

    Ok(())
}

pub fn run_bindgen(release: bool, project_name: &str) -> Result<()> {
    info!("Generating WASM bindings...");

    // Resolve target directory dynamically (handles workspaces)
    let metadata = MetadataCommand::new()
        .exec()
        .context("Failed to read cargo metadata")?;
    let target_dir = metadata.target_directory;

    let mode = if release { "release" } else { "debug" };

    // Construct path to WASM file
    let wasm_path = target_dir
        .join("wasm32-unknown-unknown")
        .join(mode)
        .join(format!("{}.wasm", project_name));

    let wasm_path_str = wasm_path.to_string();

    let dist_pkg_dir = Path::new("dist/pkg");
    if !dist_pkg_dir.exists() {
        fs::create_dir_all(dist_pkg_dir)?;
    }

    if !wasm_path.exists() {
        bail!("WASM file not found at: {}", wasm_path_str);
    }

    let status = Command::new("wasm-bindgen")
        .arg(&wasm_path)
        .arg("--out-dir")
        .arg("dist/pkg")
        .arg("--out-name")
        .arg(project_name.replace("-", "_"))
        .arg("--target")
        .arg("web")
        .arg("--no-typescript")
        .status()
        .context("Failed to run wasm-bindgen")?;

    if !status.success() {
        bail!(
            "wasm-bindgen failed. Verify that '{}' exists.",
            wasm_path_str
        );
    }

    Ok(())
}

pub fn generate_dist(project_name: &str) -> Result<()> {
    // Generate/Copy index.html
    let index_src = Path::new("index.html");
    let index_dest = Path::new("dist/index.html");

    if index_src.exists() {
        let content = fs::read_to_string(index_src)?;
        // Simple injection of the script tag if not present?
        // Trunk parses HTML. We are "Native Nexa".
        // Use a simple heuristic: standard index.html for nexa apps should have the script.
        // If the user provided one, we copy it.
        // WE MUST ensure the script loading code is there.
        // Nexa native loader:
        let script = format!(
            r#"
<script type="module">
    import init from '/{}.js';
    init();
</script>
"#,
            project_name.replace("-", "_")
        ); // wasm-bindgen uses underscores

        // If content doesn't have the script, inject it before </body> or at end
        let final_content = if !content.contains("init()") {
            if content.contains("</body>") {
                content.replace("</body>", &format!("{}{}", script, "</body>"))
            } else {
                format!("{}{}", content, script)
            }
        } else {
            content
        };

        fs::write(index_dest, final_content)?;
    } else {
        // Fallback or Error? User probably has one.
        bail!("index.html not found");
    }

    Ok(())
}
