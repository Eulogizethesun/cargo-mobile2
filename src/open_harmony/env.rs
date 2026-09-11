use crate::{
    env::{Error as CoreError, ExplicitEnv},
    os::Env as CoreEnv,
    util::cli::{Report, Reportable},
};
use std::{
    collections::HashMap,
    ffi::OsString,
    path::{Path, PathBuf},
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    CoreEnvError(#[from] CoreError),
    #[error("Have you installed DevEco Studio or the OpenHarmony command line tools? The `OHOS_HOME` environment variable isn't set, and is required. Set it to the `openharmony` SDK directory (by default `C:\\Program Files\\Huawei\\DevEco Studio\\sdk\\default\\openharmony` on Windows, `/Applications/DevEco-Studio.app/Contents/sdk/default/openharmony` on macOS, or `<command line tools>/sdk/default/openharmony` on Linux).")]
    OhosHomeNotSet,
    #[error("Have you installed the OpenHarmony SDK? The `OHOS_HOME` environment variable is set, but doesn't point to an existing directory.")]
    OhosHomeNotADir,
}

impl Reportable for Error {
    fn report(&self) -> Report {
        match self {
            Self::CoreEnvError(err) => err.report(),
            _ => Report::error("Failed to initialize OpenHarmony environment", self),
        }
    }
}

impl Error {
    pub fn sdk_issue(&self) -> bool {
        !matches!(self, Self::CoreEnvError(_))
    }
}

/// Per-OS default DevEco Studio install root. DevEco Studio isn't distributed
/// for Linux, so there is no default there — `OHOS_HOME` must point at the
/// OpenHarmony command line tools' SDK instead.
fn default_deveco_root() -> Option<PathBuf> {
    if cfg!(target_os = "macos") {
        Some(PathBuf::from("/Applications/DevEco-Studio.app/Contents"))
    } else if cfg!(target_os = "windows") {
        Some(
            std::env::var_os("DEV_ECO_STUDIO_INSTALL_PATH")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("C:\\Program Files\\Huawei\\DevEco Studio")),
        )
    } else {
        None
    }
}

#[derive(Debug, Clone)]
pub struct Env {
    pub base: CoreEnv,
    ohos_home: PathBuf,
    deveco_root: PathBuf,
}

impl Env {
    pub fn new() -> Result<Self, Error> {
        Self::from_env(CoreEnv::new()?)
    }

    /// `OHOS_HOME` must point at the `openharmony` SDK directory — by
    /// convention `<DevEco install>/sdk/default/openharmony`, with **no
    /// version segment**, for either a DevEco Studio install or the
    /// OpenHarmony command line tools. Everything else (SDK root, ohpm,
    /// hvigor, node, emulator) is derived from that shape.
    pub fn from_env(base: CoreEnv) -> Result<Self, Error> {
        let (ohos_home, deveco_root) = match std::env::var_os("OHOS_HOME") {
            Some(home) => {
                let ohos_home = PathBuf::from(home);
                if !ohos_home.is_dir() {
                    return Err(Error::OhosHomeNotADir);
                }
                // `OHOS_HOME` is `<root>/sdk/default/openharmony`, so the
                // install root is three levels up.
                let deveco_root = ohos_home
                    .parent()
                    .and_then(Path::parent)
                    .and_then(Path::parent)
                    .map(Path::to_path_buf)
                    .unwrap_or_else(|| ohos_home.clone());
                (ohos_home, deveco_root)
            }
            None => {
                let Some(deveco_root) = default_deveco_root() else {
                    return Err(Error::OhosHomeNotSet);
                };
                let ohos_home = deveco_root.join("sdk").join("default").join("openharmony");
                if !ohos_home.is_dir() {
                    return Err(Error::OhosHomeNotSet);
                }
                (ohos_home, deveco_root)
            }
        };
        Ok(Self {
            base,
            ohos_home,
            deveco_root,
        })
    }

    pub fn path(&self) -> &OsString {
        self.base.path()
    }

    /// The `openharmony` SDK directory (`OHOS_HOME`).
    pub fn ohos_home(&self) -> &Path {
        &self.ohos_home
    }

    /// The DevEco Studio / OpenHarmony command line tools install root that
    /// `OHOS_HOME` was resolved from.
    pub fn deveco_root(&self) -> &Path {
        &self.deveco_root
    }

    /// The SDK root (`<install root>/sdk`), exported as `DEVECO_SDK_HOME`.
    pub fn sdk_root(&self) -> PathBuf {
        self.deveco_root.join("sdk")
    }

    pub fn toolchains_path(&self) -> PathBuf {
        PathBuf::from(&self.ohos_home).join("toolchains")
    }

    /// Path to the `ohpm` package manager binary.
    pub fn ohpm_path(&self) -> PathBuf {
        if cfg!(target_os = "macos") {
            self.deveco_root
                .join("tools")
                .join("ohpm")
                .join("bin")
                .join("ohpm")
        } else if cfg!(target_os = "linux") {
            // DevEco Studio isn't available on Linux; the command line tools
            // put ohpm directly under the install root.
            self.deveco_root.join("ohpm").join("bin").join("ohpm")
        } else {
            self.deveco_root
                .join("tools")
                .join("ohpm")
                .join("bin")
                .join("ohpm.bat")
        }
    }

    /// Path to the Node binary bundled with DevEco Studio, used to run
    /// `hvigorw.js` when `hvigorw` isn't on `PATH`.
    pub fn hvigor_node_path(&self) -> PathBuf {
        if cfg!(target_os = "macos") {
            self.deveco_root
                .join("tools")
                .join("node")
                .join("bin")
                .join("node")
        } else {
            self.deveco_root.join("tools").join("node").join("node.exe")
        }
    }

    /// Path to the `hvigorw.js` script bundled with DevEco Studio.
    pub fn hvigorw_script_path(&self) -> PathBuf {
        self.deveco_root
            .join("tools")
            .join("hvigor")
            .join("bin")
            .join("hvigorw.js")
    }

    /// Path to the DevEco Studio emulator binary.
    pub fn emulator_path(&self) -> PathBuf {
        if cfg!(target_os = "macos") {
            self.deveco_root
                .join("tools")
                .join("emulator")
                .join("Emulator")
        } else {
            self.deveco_root
                .join("tools")
                .join("emulator")
                .join("Emulator.exe")
        }
    }
}

impl ExplicitEnv for Env {
    fn explicit_env(&self) -> HashMap<String, OsString> {
        let mut envs = self.base.explicit_env();
        envs.insert(
            "OHOS_HOME".into(),
            self.ohos_home.as_os_str().to_os_string(),
        );
        envs.insert(
            "OHOS_NDK_HOME".into(),
            self.ohos_home.as_os_str().to_os_string(),
        );
        envs.insert(
            "OHOS_BASE_SDK_HOME".into(),
            self.ohos_home.as_os_str().to_os_string(),
        );
        // `OHOS_HOME` is `<root>/sdk/default/openharmony`, so the SDK root
        // that hvigor expects in `DEVECO_SDK_HOME` is two levels up.
        envs.insert("DEVECO_SDK_HOME".into(), self.sdk_root().into_os_string());
        envs
    }
}
