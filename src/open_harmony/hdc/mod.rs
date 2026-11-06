pub mod device_list;
pub mod get_prop;

pub use self::{device_list::device_list, get_prop::get_prop};

use super::env::Env;
use crate::{env::ExplicitEnv as _, util::cli::Report, DuctExpressionExt};
use std::{ffi::OsString, str, string::FromUtf8Error};
use thiserror::Error;

pub fn hdc<U>(env: &Env, args: U) -> duct::Expression
where
    U: IntoIterator,
    U::Item: Into<OsString>,
{
    duct::cmd(env.toolchains_path().join("hdc"), args).vars(env.explicit_env())
}

#[derive(Debug, Error)]
pub enum RunCheckedError {
    #[error(transparent)]
    InvalidUtf8(#[from] FromUtf8Error),
    #[error(transparent)]
    CommandFailed(std::io::Error),
}

impl RunCheckedError {
    pub fn report(&self, msg: &str) -> Report {
        match self {
            Self::InvalidUtf8(err) => Report::error(msg, err),
            Self::CommandFailed(err) => Report::error(msg, err),
        }
    }
}
