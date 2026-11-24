# Appterisk Messenger: macOS development handoff

Prepared: 2026-11-25. Baseline: v0.5.0, Rust + Tauri 2 + Vue 3 + TypeScript.

## Current status

The owner has a Mac available for development and manual testing. This document is a setup guide and implementation handoff, not evidence of a working Mac release.

The current application is Windows-first. Account WebView creation in `src-tauri/src/accounts.rs` explicitly refuses non-Windows platforms. The bundle configuration targets NSIS/MSI, and desktop integrations use Windows-specific modules. A successful frontend build alone does not demonstrate macOS support.

macOS uses WKWebView, not WebView2. Windows browser profiles must not be copied to the Mac. Link new sessions through the official provider websites after native profile isolation has been verified.

## 1. Prepare your Mac

Install Git, Node.js 24 LTS with npm, Rust stable through the official Rust installer, and Xcode Command Line Tools. For desktop development, Tauri permits Command Line Tools without the full Xcode application.

```bash
xcode-select --install
```

Install Node.js from https://nodejs.org/ and Rust using the instructions at https://rustup.rs/. Open a new terminal afterward, then check:

```bash
sw_vers
uname -m
xcode-select -p
git --version
node --version
npm --version
rustc --version
cargo --version
```

`arm64` means Apple Silicon; `x86_64` means Intel. Build for the host architecture first. Do not claim a universal build or support for both architectures without testing them.

Start with macOS 14 or newer: Tauri's named persistent WKWebView data-store API requires macOS 14+. This is a proposed porting minimum, not a tested compatibility claim.

## 2. Get the source

```bash
git clone https://github.com/Appterisk-Solution/appterisk-messenger.git
cd appterisk-messenger
git switch -c feature/macos-support
npm ci
npm run typecheck
npm run lint
npm test
```

If the folder already exists, inspect `git status` and preserve local changes before updating. Do not reset or replace an existing working copy blindly.

## 3. Give your Mac coding agent this task

Copy the following instructions into your coding agent on the Mac:

> Read MACOS_HANDOFF.md, README.md, ARCHITECTURE.md, CODE_SIGNING.md and the current release verification. Implement an experimental macOS port of Appterisk Messenger on feature/macos-support. Preserve the Windows implementation and all existing account IDs, metadata, profile protection and trusted-shell IPC rules. Use Rust, Tauri 2 and native WKWebView; do not introduce Electron, CEF, an external backend or unofficial messaging APIs. Inspect the pinned Tauri/Wry implementation before selecting platform APIs. Implement and verify independent persistent website data stores keyed by stable account UUIDs on macOS 14+. Never fall back to one shared data store for multiple accounts. Add macOS lifecycle, external links, permission handling, downloads, notifications, tray and shortcuts as supported; clearly disable unsupported functionality. Keep the Windows updater feed and NSIS installer logic isolated from macOS. First prove synthetic storage isolation and persistence, then ask the owner to verify real provider logins through three complete restarts. Do not claim provider compatibility from synthetic tests. Produce a local experimental app/DMG for the host architecture, report actual checks and leave unverified items pending. Do not delete profiles, inspect authentication secrets, weaken Gatekeeper, or fabricate signing/notarization results.

## 4. Porting requirements

- **Persistent account stores:** review `accounts.rs`, `probe.rs` and `profile_paths.rs`. Windows `data_directory` is not a WKWebView profile-isolation solution. Tauri exposes `data_store_identifier([u8; 16])` on macOS 14+; derive it from the existing account UUID and reuse it after every restart. Verify the pinned library API and runtime behavior. Keep cookies, IndexedDB, local storage, service workers and cache independent.
- **Shell security:** WKWebView's local custom-protocol origin differs from Windows. Inspect `probe::trusted`, capabilities and navigation handlers. Allow only the actual trusted local shell origin and label; never grant updater, filesystem or account IPC to remote messaging pages.
- **Lifecycle:** use normal macOS window/application shutdown and native lifecycle behavior. Do not force-kill web content processes or reuse Windows COM exit checks on macOS. Keep inactive account WebViews alive where supported and resize only the selected visible view.
- **Desktop features:** keep Windows implementations behind platform guards. Add modular macOS implementations or explicit unavailable states. Use Command-based shortcuts where appropriate; verify menus, tray, downloads, external links and notification permissions on the Mac.
- **Bundle configuration:** introduce a macOS-specific configuration for app/DMG targets and the tested minimum OS. Keep the production Windows identifier and NSIS/MSI configuration intact. A dedicated experimental Mac identifier may be used before real accounts are linked; do not change it between persistence tests.
- **Updates:** the present Rust updater accepts Windows x64 NSIS artifacts and must not try to install them on macOS. Disable automatic Mac installation until a separate signed Mac artifact/feed/lifecycle has been implemented and tested. Do not reuse or overwrite the Windows feeds.
- **Privacy:** never print, export or copy messages, passwords, provider tokens, encryption keys or browser authentication databases. Collect only non-sensitive diagnostics.

## 5. Run and build after implementing the port

The commands below are validation targets. They are not expected to produce a functioning Mac account manager from the unmodified Windows baseline.

```bash
npm run typecheck
npm run lint
npm test
npm run cargo -- fmt --manifest-path src-tauri/Cargo.toml --check
npm run cargo -- clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
npm run cargo -- test --manifest-path src-tauri/Cargo.toml
npm run tauri -- dev
```

Existing native isolation/manager/desktop fixtures are Windows-specific; adapt equivalent Mac tests before claiming those checks passed on macOS.

For an explicitly local, ad-hoc-signed experimental bundle after the port works, use a command-scoped identity and disable updater artifact generation for this test only:

```bash
APPLE_SIGNING_IDENTITY="-" npm run tauri -- build --bundles app,dmg --config '{"bundle":{"createUpdaterArtifacts":false}}'
```

An ad-hoc signature does **not** establish an Apple-verified publisher or notarization. Do not use the Windows-only `build:signed` helper on macOS, copy the production updater private key to the Mac, or upload it to GitHub to make a local test build work.

Typical default output directories are `src-tauri/target/release/bundle/macos/` and `src-tauri/target/release/bundle/dmg/`. Explicit Rust targets can add an architecture directory; use the actual CLI output instead of assuming an artifact exists.

## 6. Gatekeeper and local testing

A locally built app and a browser-downloaded DMG may behave differently under Gatekeeper. A manual override is not proof of safe distribution or application compatibility. This guide does not disable Gatekeeper, remove quarantine attributes or change system security settings. If macOS blocks the test app, inspect the exact warning and follow Apple's supported guidance yourself; do not override a known-malware warning.

Normal direct distribution requires an Apple-issued Developer ID signature and notarization. SignPath's Windows application does not supply Apple signing. Keep unsigned/ad-hoc test artifacts labeled experimental and separate from production releases.

## 7. Acceptance checklist on the Mac

Record the source commit, macOS version, architecture, app version and actual results. Mark each item PASS, FAIL or NOT TESTED.

1. Launch the app; confirm settings and dialogs stay above account WebViews.
2. Seed three independent synthetic native stores; verify cookies, localStorage, IndexedDB, blobs and other supported durable storage across three full app quits/reopens.
3. Verify identical storage keys can hold different values in the three stores without leakage.
4. Add three WhatsApp accounts and link each normally only after isolation passes.
5. Switch accounts: only one WebView is visible; switching does not unnecessarily reload it.
6. Quit the entire app using its Quit menu or Command-Q, reopen and check all three logins. Repeat three times. Closing the window alone may leave the process running.
7. Rename/reorder accounts; verify IDs, order, preferences and last active account after restart.
8. Remove one disposable account with confirmation; verify the other stores and logins are unaffected. Retain existing profile data unless the owner explicitly requests its deletion.
9. Resize, minimize and restore; check active content bounds.
10. Test notifications, permissions, file downloads, external links, tray/menu behavior and shortcuts individually. Record unavailable features honestly.
11. Test Telegram and Discord independently if enabled; do not infer acceptance from WhatsApp.
12. Verify remote provider pages cannot invoke privileged shell commands.

Real authentication tests require owner confirmation. Do not attach browser profiles, private chats or authentication data to the test report.

## Official references

- [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/)
- [Tauri native WebView builder and data-store identifiers](https://docs.rs/tauri/latest/tauri/webview/struct.WebviewBuilder.html#method.data_store_identifier)
- [Tauri macOS signing](https://v2.tauri.app/distribute/sign/macos/)
- [Apple Developer ID](https://developer.apple.com/developer-id/)
- [Apple guidance for opening apps safely](https://support.apple.com/en-us/102445)
