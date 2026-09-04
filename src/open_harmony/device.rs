use super::{config::Config, env::Env, target::Target};
use crate::{
    env::ExplicitEnv,
    open_harmony::{hap, hdc},
    opts::{NoiseLevel, Profile},
    util::{
        cli::{Report, Reportable},
        last_modified,
    },
    DuctExpressionExt,
};
use std::fmt::{self, Display};
use std::time::Duration;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RunError {
    #[error(transparent)]
    HapError(hap::HapError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

impl Reportable for RunError {
    fn report(&self) -> Report {
        match self {
            Self::HapError(err) => err.report(),
            Self::Io(err) => Report::error("IO error", err),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum StacktraceError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

impl Reportable for StacktraceError {
    fn report(&self) -> Report {
        match self {
            Self::Io(err) => Report::error("IO error", err),
        }
    }
}

#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Device<'a> {
    id: String,
    name: String,
    model: String,
    target: &'a Target<'a>,
}

impl Display for Device<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)?;
        if self.model != self.name {
            write!(f, " ({})", self.model)?;
        }
        Ok(())
    }
}

impl<'a> Device<'a> {
    pub(super) fn new(id: String, name: String, model: String, target: &'a Target<'a>) -> Self {
        Self {
            id,
            name,
            model,
            target,
        }
    }

    pub fn target(&self) -> &'a Target<'a> {
        self.target
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn model(&self) -> &str {
        &self.model
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    fn hdc(&self, env: &Env) -> duct::Expression {
        hdc::hdc(env, ["-t", &self.id])
    }

    fn wait_device_boot(&self, env: &Env) {
        // The Android original bounds each poll with a 3s timeout; `hdc` needs
        // the same treatment. Every round sleeps too, so a hung or failing
        // `hdc` can't burn the CPU in a tight loop, and a bounded number of
        // consecutive failures gives up instead of spinning forever.
        const HDC_CALL_TIMEOUT: Duration = Duration::from_secs(3);
        const RETRY_DELAY: Duration = Duration::from_secs(2);
        const MAX_FAILURES: usize = 10;

        // Polls the boot property once. `Some(booted)` when `hdc` ran within
        // the timeout and exited cleanly, `None` when it failed or timed out
        // (the timed-out child is killed so a hung `hdc` doesn't linger).
        fn poll_boot(cmd: duct::Expression, timeout: Duration) -> Option<bool> {
            let handle = cmd.start().ok()?;
            let output = match handle.wait_timeout(timeout) {
                Ok(Some(output)) => output,
                Ok(None) | Err(_) => {
                    let _ = handle.kill();
                    return None;
                }
            };
            if !output.status.success() {
                return None;
            }
            let stdout = String::from_utf8_lossy(&output.stdout);
            // `ohos.boot.time.init` only parses as a number once boot finishes.
            Some(stdout.trim().parse::<usize>().is_ok())
        }

        let mut failures = 0usize;
        loop {
            let cmd = self
                .hdc(env)
                .stderr_capture()
                .stdout_capture()
                .before_spawn(move |cmd| {
                    cmd.args(["shell", "param", "get", "ohos.boot.time.init"]);
                    Ok(())
                });
            match poll_boot(cmd, HDC_CALL_TIMEOUT) {
                Some(true) => break,
                // Polled fine, but the device is still booting — not a failure.
                Some(false) => {}
                None => {
                    failures += 1;
                    if failures >= MAX_FAILURES {
                        log::warn!(
                            "Failed to detect device boot after {MAX_FAILURES} attempts; continuing anyway"
                        );
                        break;
                    }
                }
            }
            std::thread::sleep(RETRY_DELAY);
        }
    }

    fn build_hap(
        &self,
        config: &Config,
        env: &Env,
        noise_level: NoiseLevel,
        profile: Profile,
    ) -> Result<(), hap::HapError> {
        hap::build(config, env, noise_level, profile)?;
        Ok(())
    }

    fn install_hap(&self, config: &Config, env: &Env) -> Result<(), std::io::Error> {
        let hap_path = hap::haps_paths(config)
            .into_iter()
            .reduce(last_modified)
            .unwrap();

        self.hdc(env)
            .before_spawn(move |cmd| {
                cmd.args(["install", "-r"]);
                cmd.arg(&hap_path);
                Ok(())
            })
            .dup_stdio()
            .start()?
            .wait()?;

        Ok(())
    }

    fn wake_screen(&self, _env: &Env) -> std::io::Result<()> {
        // TODO: seems like there's no equivalent to `adb shell input keyevent KEYCODE_WAKEUP`
        // in hdc (a keyevent can be sent with `hdc shell uitest uiInput keyEvent Power`)
        Ok(())
    }

    // see https://developer.huawei.com/consumer/en/doc/harmonyos-guides/web-debugging-with-devtools
    fn setup_devtools_port_forwarding_async(&self, env: &Env, pid: &str) {
        let explicit_env = env.explicit_env();
        let hdc_path = env.toolchains_path().join("hdc");
        let pid = pid.to_string();

        std::thread::spawn(move || {
            const MAX_ATTEMPTS: usize = 10;
            let mut retries = 0;

            // wait for the remote devtools socket to be opened
            let expected_open_socket_content = format!("@webview_devtools_remote_{pid}");
            loop {
                std::thread::sleep(std::time::Duration::from_secs(1));
                let Ok(opened_sockets) = duct::cmd(&hdc_path, ["shell", "cat", "/proc/net/unix"])
                    .vars(explicit_env.clone())
                    .read()
                else {
                    break;
                };
                if opened_sockets.contains(&expected_open_socket_content) {
                    break;
                }

                retries += 1;
                if retries >= MAX_ATTEMPTS {
                    log::error!(
                        "Could not setup port forwarding for devtools. Make sure you are running setWebDebuggingAccess(true). See https://developer.huawei.com/consumer/en/doc/harmonyos-guides/web-debugging-with-devtools for more information."
                    );
                    return;
                }
            }

            // forward the remote devtools socket to the local port 9222
            // so Chrome can connect to it
            let _ = duct::cmd(
                &hdc_path,
                [
                    "fport",
                    "tcp:9222",
                    &format!("localabstract:webview_devtools_remote_{pid}"),
                ],
            )
            .vars(explicit_env)
            .dup_stdio()
            .start();
        });
    }

    #[allow(clippy::too_many_arguments)]
    pub fn run(
        &self,
        config: &Config,
        env: &Env,
        noise_level: NoiseLevel,
        profile: Profile,
    ) -> Result<duct::Handle, RunError> {
        self.build_hap(config, env, noise_level, profile)
            .map_err(RunError::HapError)?;
        if self.model.starts_with("emulator") {
            self.wait_device_boot(env);
        }
        self.install_hap(config, env).map_err(RunError::Io)?;

        let identifier = config.app().identifier().to_string();
        self.hdc(env)
            .before_spawn(move |cmd| {
                cmd.args([
                    "shell",
                    "aa",
                    "start",
                    "-b",
                    &identifier,
                    "-a",
                    "EntryAbility",
                ]);
                Ok(())
            })
            .dup_stdio()
            .start()?
            .wait()?;

        let _ = self.wake_screen(env);

        let stdout = loop {
            let cmd = duct::cmd(
                env.toolchains_path().join("hdc"),
                ["shell", "pidof", "-s", config.app().identifier()],
            )
            .vars(env.explicit_env())
            .stderr_capture()
            .stdout_capture();
            let handle = cmd.start()?;
            if let Ok(out) = handle.wait() {
                if out.status.success() {
                    break String::from_utf8_lossy(&out.stdout).into_owned();
                }
            }
            std::thread::sleep(std::time::Duration::from_secs(2));
        };
        let pid = stdout.trim().to_string();

        self.setup_devtools_port_forwarding_async(env, &pid);

        let mut logcat = duct::cmd(env.toolchains_path().join("hdc"), ["hilog", "-v", "color"])
            .vars(env.explicit_env())
            .dup_stdio();

        let logcat_filter_specs = config.logcat_filter_specs().to_vec();
        logcat = logcat.before_spawn(move |cmd| {
            if !pid.is_empty() {
                cmd.args(["--pid", &pid]);
            }
            cmd.args(&logcat_filter_specs);
            Ok(())
        });
        logcat.start().map_err(Into::into)
    }

    pub fn stacktrace(&self, config: &Config, env: &Env) -> Result<(), StacktraceError> {
        // The `.so` lives under the active entry module's `libs/` dir, where
        // `ohrs build --dist` put it (`<dist>/<abi>/lib<name>.so`, e.g.
        // `arm64-v8a`).
        let lib_path = config
            .so_dist_dir()
            .join(self.target.abi)
            .join(config.so_name());

        // -x = print and exit
        let hilog_command = hdc::hdc(env, ["-t", &self.id])
            .before_spawn(move |cmd| {
                cmd.args(["hilog", "-x"]);
                cmd.arg("-sym");
                cmd.arg(&lib_path);
                Ok(())
            })
            .dup_stdio();

        if hilog_command.start()?.wait().is_err() {
            println!("  -- no stacktrace --");
        }
        Ok(())
    }
}
