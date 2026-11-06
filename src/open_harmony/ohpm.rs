use std::path::PathBuf;

use crate::{
    open_harmony::{config::Config, env::Env},
    DuctExpressionExt,
};

pub fn install(config: &Config, env: &Env) -> std::io::Result<()> {
    log::info!("Installing packages with ohpm...");
    let mut ohpm_path = env.ohpm_path();
    if !ohpm_path.exists() {
        log::warn!(
            "ohpm not found in {}, expecting ohpm to be available in PATH...",
            ohpm_path.display()
        );
        ohpm_path = PathBuf::from("ohpm");
    }

    duct::cmd(ohpm_path, ["install"])
        .dir(config.project_dir())
        .dup_stdio()
        .start()?
        .wait()?;
    Ok(())
}
