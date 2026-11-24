> Current 0.2.0 status: The user confirmed three distinct 0.1.2 accounts passed three restarts before production implementation. The account manager is now implemented; see [0.2.0 release verification](RELEASE_0.2.0.md). Earlier pending statements below are historical. The user also separately confirmed all three adopted accounts stayed logged in through three restarts on the installed 0.2.0 manager.

> Current status (0.1.2, 2026-11-24): The Windows browser-profile path representation was fixed. The user confirmed login persistence through three restarts in Edge, native WebView2, minimal Tauri and the main application. Earlier pending/failure entries below are historical. Three distinct phone accounts have not been confirmed. Production UI and SQLite remain outside this fix. See [the release investigation](AUTH_PERSISTENCE_0.1.2.md).

# Execution report

## 0.1.1 session-persistence investigation

The user reported real WhatsApp login persistence failure on 0.1.0, despite the phone still listing the linked device. Production work remains blocked. Implemented explicit persistent mode, effective native-folder/InPrivate validation, view reuse, native controller close with retained environment exit subscriptions, coordinated window/app shutdown and metadata-only browser health logging. Existing profile directories were preserved. Exact causal attribution and real-session success remain unverified pending user retest; see SESSION_PERSISTENCE.md.

Current checks pass: frontend typecheck/lint/build, three frontend tests, four Rust tests, rustfmt and Clippy (`-D warnings`). The synthetic isolation test now passes three restart cycles and verifies normal browser process exit for each account environment (run `1d259b34-5686-46fb-aab2-a386fcd167ca`). Three measured actual-origin metadata cycles also showed matching existing folders, InPrivate=false, cookie/storage metadata and normal account-browser exits; this does not assert authenticated login continuity. Warning 1412 still occurs after normal account-browser exit acknowledgments at final shell teardown.

`npm run tauri build` passed: unsigned `Appterisk Messenger_0.1.1_x64_en-US.msi` and `Appterisk Messenger_0.1.1_x64-setup.exe` were produced. The 0.1.1 release executable was opened with `--resume-whatsapp` for the user-held-device retest. Edge comparison helper `npm run test:edge` uses a separate fixed normal profile and does not copy authentication data; linked-device comparison is pending user participation.

Primary changed files: `src-tauri/src/{lib,probe,health,windows_profiles,diagnostics}.rs`, `src-tauri/diagnostics/{origin-metadata,error-counts}.js`, `src/App.vue`, `src/services/probe.ts`, `scripts/{isolation,edge-comparison}.mjs`, manifests/lockfiles, ESLint configuration and investigation documentation. No Electron, backend, SQLite account manager or production UI was introduced.

Phase 1 follow-up: the user confirmed three-account linked-session restart tests are not yet performed, so the requested production implementation remains gated. Added a debug-only native diagnostic using dedicated disposable profiles and `npm run test:isolation`; all three profiles passed cookie/localStorage/IndexedDB isolation, persistence across two fresh application processes, and local child IPC rejection. Evidence and exact device steps are recorded in PHASE1_GATE.md. Added a reusable Windows-native environment module and Cargo npm wrapper. Typecheck, lint, three Vue tests, three Rust tests, rustfmt check and Clippy with `-D warnings` pass. The optimized release build and unsigned Windows MSI/NSIS packaging pass. These results apply to the compatibility probe, not a completed Phase 1 application.

Update after build-setup repair: Windows debug compilation and app launch now pass. The local 32-bit .NET native-image failure was repaired, Build Tools/Windows SDK installed (exit 0), and `npm run tauri dev` compiled and launched the visible application. Three native provider login pages were observed. See BUILD_SETUP.md for the repair details, including the separate unsuccessful DISM attempt. Earlier blocked results below describe the initial run; native unit tests/Clippy, release packaging and real account isolation/persistence remain pending.

## Delivered

Official create-tauri-app Vue/TypeScript scaffold, three persistent Windows profile probe, serialized native lifecycle commands, trusted-caller validation, exact-origin navigation policy, permission-free child capabilities, shell CSP, responsive diagnostic UI, guide overlay handling, local storage/cookie/IPC fixture, frontend and Rust policy tests, Windows CI and compatibility documentation.

Primary source changes: `src/App.vue`, `src/style.css`, `src/services/probe.ts`, `src-tauri/src/{lib,probe}.rs`, `src-tauri/{Cargo.toml,tauri.conf.json,capabilities/default.json}`, `public/isolation.*`, `tests/probe.test.ts`, ESLint/Vitest configuration, npm scripts, both lockfiles, README, docs and `.github/workflows/checks.yml`. The workspace was empty before scaffolding. No existing work was overwritten and no Git repository was initialized.

## Commands and results

- `npm create tauri-app@latest . -- --template vue-ts --manager npm --identifier com.appterisk.messenger --yes`: scaffold created.
- `npm install` and dependency setup: completed; package-lock.json generated.
- `npm run typecheck`: passed.
- `npm run lint`: passed after fixing the scaffold's loose Vue declaration.
- `npm test`: 3 frontend tests passed (error/retry UI, logical DPI geometry and structured errors).
- `npm run build`: passed. HTML bundle only, not a desktop executable.
- `npm audit --omit=dev`: zero production findings. Full audit retains four moderate findings in test utilities' transitive glob/minimatch dependencies; `npm audit fix` did not resolve them.
- Downloaded Rust 1.100.0 into `%TEMP%/appterisk-rust-tools`, with isolated CARGO_HOME/RUSTUP_HOME and no system PATH change. Added rustfmt/Clippy there.
- `cargo fmt --manifest-path src-tauri/Cargo.toml`: completed. Formatting check passed subsequently.
- `cargo generate-lockfile --manifest-path src-tauri/Cargo.toml`: completed; Tauri 2.12.7/wry 0.57.3 resolved.
- `cargo check --manifest-path src-tauri/Cargo.toml`: failed linking dependency build scripts. Missing Microsoft linker/SDK; unrelated coreutils `link.exe` is on PATH. Application Rust typechecking was not reached.
- Native Clippy/tests and installer build: pending the same prerequisites; not claimed passed. CI includes all required checks.

## Blockers and remaining work

Install MSVC C++ tools/Windows SDK and a persistent Rust stable toolchain, then run the README commands. The temporary Rust installation can be reused in the current environment by setting CARGO_HOME/RUSTUP_HOME to its subdirectories and prepending its cargo/bin directory to process PATH; it is not a permanent machine setup.

The mandatory three-account QR/login/isolation/restart gate needs user-held linked devices. See COMPATIBILITY.md for exact steps. Media, offline recovery, DPI, IPC denial, performance and installer tests are pending. Downloads/permissions are restricted rather than reported working. macOS/Linux adapters are disabled.

Phase 1 remains incomplete: SQLite/migrations, full account IPC, CRUD/reorder/switch/delete, settings/themes and resource-aware lazy lifecycle have not been built. This follows the specification's compatibility-first gate; do not mistake the diagnostic fixture for an account manager. No later phases were implemented.

Exact development/build commands after installing prerequisites: `npm install`, `npm run tauri dev`, `npm run tauri build`.
