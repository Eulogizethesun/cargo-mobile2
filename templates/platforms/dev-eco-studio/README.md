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

Known, accepted drift (verified 2026-10-22):

- Base class: `EntryAbility.ets` here extends `RustAbility`, which
  `@ohos-rs/ability` renamed to `NativeAbility` in March 2026 — the current
  package (1.0.0-beta.1) no longer exports `RustAbility`. This template pins
  `@ohos-rs/ability` 0.2.1 from the registry and has not tracked the rename;
  the tauri-cli template extends `NativeAbility` and additionally wires the
  bridge plugins (`allBridgePlugins`) and the Tauri plugin manager.
- Appearance: `EntryAbility.ets` here calls `setWindowLayoutFullScreen(false)`
  and sets no window background or system-bar colors; the tauri-cli template
  opts into the full-screen layout with a white window background and
  transparent system bars.
- Permissions: `module.json5` here declares only `ohos.permission.INTERNET`;
  the tauri-cli template declares the full baseline (5 permissions on the
  mobile form, 8 on the desktop form — see its README).
- Launch type: this template omits `launchType` (the OpenHarmony default is
  `singleton`); the tauri-cli template writes `singleton` for the mobile form
  and `specified` (multi-UIAbility window support, with an
  `EntryAbilityStage`) for the desktop form.

Aligning the two templates is tracked in the tauri repository
(Eulogizethesun/tauri): #230 tracks this drift, with the appearance-injection
rework in #224 planned as the unification wave (#227's dangling demo page
was resolved in both templates by replacing it with a content-free
placeholder page — hvigor's main_pages.json schema requires at least one
registered page). Until then: do not port changes into this template without
checking the tauri-cli template first.
