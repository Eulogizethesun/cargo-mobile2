# DevEco Studio platform template (minimal scaffold)

This is the OpenHarmony platform template `cargo mobile init` generates DevEco
Studio projects from. It is deliberately minimal: it only aims to produce a
buildable bare-bones project, and it is NOT the authoritative OpenHarmony
template for Tauri apps.

The authoritative template lives in the tauri repository at
`crates/tauri-cli/templates/mobile/open-harmony/`. `tauri-cli` embeds it
(`include_dir`) and scaffolds exclusively from it, using `cargo-mobile2` only
as a library for OpenHarmony tooling — never this pack. Its appearance and
permission baselines are the source of truth; this template intentionally
defers to them and is expected to lag behind.

Known, accepted drift (re-verified 2026-11-16 against
`crates/tauri-cli/templates/mobile/open-harmony/` in the tauri repository):

- Base class: `EntryAbility.ets` here extends `RustAbility`, which
  `@ohos-rs/ability` renamed to `NativeAbility` in March 2026 — the current
  package (1.0.0-beta.1) no longer exports `RustAbility`. This template pins
  `@ohos-rs/ability` 0.2.1 from the registry and has not tracked the rename;
  the tauri-cli template extends `NativeAbility` (shipped as a Handlebars
  template, `EntryAbility.ets.hbs`, rendered at init) and additionally wires
  the bridge plugins (`allBridgePlugins`) and the Tauri plugin manager.
- Appearance: `EntryAbility.ets` here calls `setWindowLayoutFullScreen(false)`
  and sets no window background or system-bar properties (theme-following
  defaults); the tauri-cli template opts into the full-screen layout
  (`setWindowLayoutFullScreen(true)`), makes both system bars transparent,
  derives their content color at generation time from tauri.conf's first
  `app.windows` background color (#224), and sets the window background color
  only when that value is explicitly configured — the unconditional white
  background it once hardcoded is gone, and unconfigured apps keep the
  theme-following system default.
- Permissions: `module.json5` here declares only `ohos.permission.INTERNET`;
  the tauri-cli template declares the full baseline (5 permissions on the
  mobile form, 8 on the desktop form — see its README).
- Launch type: this template omits `launchType` (the OpenHarmony default is
  `singleton`); the tauri-cli template writes `singleton` for the mobile form
  and `specified` (multi-UIAbility window support, with an
  `EntryAbilityStage`) for the desktop form.
- App identity (`AppScope/app.json5`): here `versionCode`/`versionName` are
  hardcoded defaults (`1` / `"1.0.0"`) and `vendor` uses a different
  placeholder (`{{app.publisher}}`); the tauri-cli template injects all
  three from config (`{{bundle.open-harmony.version-code}}`, `{{version}}`,
  `{{bundle.publisher}}`).
- Project modules (root `build-profile.json5`): the tauri-cli pack
  additionally registers a third module `tauri` (the Rust crate, as a
  DevEco module); this pack declares only `entry_mobile` and `entry_desktop`.
- Rust build hook (`hvigorfile.ts` in both entry forms): here a
  `cargoMobilePlugin` calls `cargo open-harmony build` directly (hardcoded
  form, `--release` profile sync); the tauri-cli pack's `tauriPlugin`
  invokes the tauri binary through placeholders and skips itself when the
  CLI drives the build (`TAURI_OHOS_SKIP_DEVECO_SCRIPT`). The root
  `hvigorfile.ts` differs only in formatting.
- Dependency wiring (`oh-package.json5` in both entry forms): here
  `@ohos-rs/ability` is pinned at 0.2.1 from the registry; the tauri-cli
  pack references it via a local `file:` HAR and adds
  `@tauri/app: file:../tauri`.
- Strings (`string.json` in both entry forms): here `module_desc` and
  `EntryAbility_desc` are literal defaults; the tauri-cli pack substitutes
  `{{bundle.short-description}}` and adds the `reason_print` string backing
  its PRINT permission.
- Menu bar colors: the tauri-cli pack's desktop-form `color.json` (base and
  dark) adds the `menubar_*` color resources; this pack has none (the
  mobile form's `color.json` matches).
- Placeholder page (`Index.ets`): the code is identical; only the
  explanatory comment differs (this pack cites the RustAbility 0.2.1 pin,
  the tauri-cli pack the NativeAbility rename).
- Lock files (`oh-package-lock.json5` in both entry forms): content-identical
  except for the trailing newline, which the tauri-cli pack omits. The root
  `oh-package.json5` and `oh-package-lock.json5` are identical outright.

This list documents the drift; it does not change any template file —
closing the gap means synchronizing the templates, which stays deferred.
Apart from the two pack READMEs themselves (which differ by design),
every file not listed above is byte-identical between the packs.

Aligning the two templates is tracked in the tauri repository
(Eulogizethesun/tauri): #230 tracks this drift — #224's appearance rework
(background color applied only on explicit config, system-bar content
color derived at generation time) has landed on the tauri-cli side, while
the categories above remain open (#227's dangling demo page was resolved
in both templates by replacing it with a content-free placeholder page —
hvigor's main_pages.json schema requires at least one registered page).
Until then: do not port changes into this template without checking the
tauri-cli template first.
