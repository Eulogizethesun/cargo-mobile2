use crate::{
    open_harmony::env::Env,
    util::cli::{Report, Reportable},
};
use std::str;
use std::time::Duration;
use thiserror::Error;

use super::hdc;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to run `hdc shell param get {prop}`: {source}")]
    LookupFailed {
        prop: String,
        source: super::RunCheckedError,
    },
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

impl Error {
    fn prop(&self) -> &str {
        match self {
            Self::LookupFailed { prop, .. } => prop,
            Self::Io(_) => unreachable!(),
        }
    }
}

impl Reportable for Error {
    fn report(&self) -> Report {
        let msg = format!("Failed to run `hdc shell param get {}`", self.prop());
        match self {
            Self::LookupFailed { source, .. } => source.report(&msg),
            Self::Io(err) => Report::error("IO error", err),
        }
    }
}

pub fn get_prop(env: &Env, serial_no: &str, prop: &str) -> Result<String, Error> {
    let prop_ = prop.to_string();
    let handle = hdc(env, ["-t", serial_no])
        .before_spawn(move |cmd| {
            cmd.args(["shell", "param", "get", &prop_]);
            Ok(())
        })
        .stdin_file(os_pipe::dup_stdin().unwrap())
        .stdout_capture()
        .stderr_capture()
        .start()?;

    let output = handle
        .wait_timeout(Duration::from_secs(3))
        .and_then(|output| {
            output.ok_or(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "`hdc shell param get` timed out",
            ))
        })?;
    String::from_utf8(output.stdout.clone())
        .map(|stdout| stdout.trim().to_string())
        .map_err(|source| Error::LookupFailed {
            prop: prop.to_owned(),
            source: super::RunCheckedError::InvalidUtf8(source),
        })
}
