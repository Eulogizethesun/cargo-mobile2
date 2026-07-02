use std::path::PathBuf;

use colored::Colorize;
use thiserror::Error;

use super::{config::Config, env::Env};
use crate::{
    opts::{NoiseLevel, Profile},
    util::{
        cli::{Report, Reportable},
        hvigorw, prefix_path,
    },
};

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Failed to assemble App: {0}")]
    AssembleFailed(#[from] std::io::Error),
    #[error("No .app output found under {0}")]
    NotFound(PathBuf),
}

impl Reportable for AppError {
    fn report(&self) -> Report {
        match self {
            Self::AssembleFailed(err) => Report::error("Failed to assemble App", err),
            Self::NotFound(dir) => Report::error(
                "Failed to assemble App",
                format!("no .app found under {}", dir.display()),
            ),
        }
    }
}

/// Project-level output dir holding the assembled `.app`
/// (`<projectName>-default-*.app`).
fn output_dir(config: &Config) -> PathBuf {
    prefix_path(config.project_dir(), "build/outputs/default")
}

/// All `.app` artifacts produced by `assembleApp` (signed + unsigned), scanned
/// from the output dir rather than matched by name so we don't have to know the
/// project name up front.
pub fn app_paths(config: &Config) -> Vec<PathBuf> {
    let dir = output_dir(config);
    std::fs::read_dir(&dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("app") {
                Some(path)
            } else {
                None
            }
        })
        .collect()
}

/// Assemble the multi-Entry `.app` via `hvigorw assembleApp` and return the
/// built `.app` path.
///
/// Unlike `hap::build`, this is a project-level task (no `--mode module`):
/// hvigor orchestrates `PackageApp` + `MakeProjectPackInfo` + `SignApp` and
/// emits the `.app` at `build/outputs/default/<projectName>-default-*.app`. The
/// Rust `.so` for each entry must already be compiled by the CLI
/// (`first_target.build` per form) — the `tauriPlugin` is skipped via
/// `TAURI_OHOS_SKIP_DEVECO_SCRIPT`, so this does not re-enter the CLI.
pub fn build(
    config: &Config,
    env: &Env,
    noise_level: NoiseLevel,
    profile: Profile,
) -> Result<PathBuf, AppError> {
    super::ohpm::install(config, env)?;

    let build_mode = profile.as_str().to_lowercase();

    let hvigor_args = vec![
        "assembleApp".to_string(),
        "--parallel".to_string(),
        "--incremental".to_string(),
        "-p".to_string(),
        format!("buildMode={build_mode}"),
    ];

    hvigorw(config, env)
        .before_spawn(move |cmd| {
            cmd.args(&hvigor_args).arg(match noise_level {
                NoiseLevel::Polite => "--info",
                NoiseLevel::LoudAndProud | NoiseLevel::FranklyQuitePedantic => "--debug",
            });
            Ok(())
        })
        .start()
        .inspect_err(|err| {
            if err.kind() == std::io::ErrorKind::NotFound {
                log::error!("`hvigorw` not found. Make sure you have the OpenHarmony command line tools installed and added to your PATH");
            }
        })?
        .wait()?;

    // Pick the unsigned artifact by name (the one assembleApp just produced).
    // No mtime heuristic, no fallback to a stale `-signed.app` — assembleApp
    // always emits an unsigned `.app`, so its absence means the build failed
    // and we should surface `NotFound` rather than hand back a stale file.
    let paths = app_paths(config);
    paths
        .iter()
        .find(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.contains("-unsigned"))
                .unwrap_or(false)
        })
        .cloned()
        .ok_or_else(|| AppError::NotFound(output_dir(config)))
}

pub mod cli {
    use super::*;

    pub fn build(
        config: &Config,
        env: &Env,
        noise_level: NoiseLevel,
        profile: Profile,
    ) -> Result<(), AppError> {
        println!("Building App...\n");

        let output = super::build(config, env, noise_level, profile)?;

        println!("\nFinished building App:");
        println!("    {}", output.to_string_lossy().green());
        Ok(())
    }
}
