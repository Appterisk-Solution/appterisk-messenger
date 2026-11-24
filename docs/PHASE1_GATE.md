> Current 0.2.0 status: The user confirmed three distinct 0.1.2 accounts passed three restarts before production implementation. The account manager is now implemented; see [0.2.0 release verification](RELEASE_0.2.0.md). Earlier pending statements below are historical. The user also separately confirmed all three adopted accounts stayed logged in through three restarts on the installed 0.2.0 manager.

> Current status (0.1.2, 2026-11-24): The Windows browser-profile path representation was fixed. The user confirmed login persistence through three restarts in Edge, native WebView2, minimal Tauri and the main application. Earlier pending/failure entries below are historical. Three distinct phone accounts have not been confirmed. Production UI and SQLite remain outside this fix. See [the release investigation](AUTH_PERSISTENCE_0.1.2.md).

# Phase 1 prerequisite record

The production implementation is explicitly gated on session isolation and real login persistence. The user subsequently performed the real restart test and reported **failure on 0.1.0**. The 0.1.1 lifecycle changes require a real-session retest; see SESSION_PERSISTENCE.md. The production UI, SQLite account manager and desktop features remain blocked.

## Automated native evidence

On this Windows machine, run `9d1cba91-04c0-4c3e-a350-9bdeedfdd082` passed the following in two separate application processes:

- Three native child WebViews, each with its own persistent browser data directory.
- Different values for the same cookie, localStorage key and IndexedDB record remained independent.
- All three values survived the first process exiting and the second process reopening the profiles.
- Each permission-free child failed to invoke the privileged `reload_probe` command. The test requires an authorization denial (`FORBIDDEN` or a capability rejection), rather than treating any command failure as success.

The fixture uses the local bundled origin. This does not prove remote-origin IPC rejection, provider login persistence, background delivery, notifications, media support or production readiness. The WebView2 engine emitted a shutdown warning about unregistering Chrome_WidgetWin_0 (1412); both diagnostic stages exited successfully and storage assertions passed.

Reports: `%LOCALAPPDATA%/com.appterisk.messenger/diagnostic-runs/9d1cba91-04c0-4c3e-a350-9bdeedfdd082/{seed,verify}.json`.

## Available checks completed

- `npm run typecheck`, `npm run lint`, `npm test`: passed; three frontend tests.
- `npm run cargo -- fmt --manifest-path src-tauri/Cargo.toml --check`: passed.
- `npm run cargo -- clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`: passed.
- `npm run cargo -- test --manifest-path src-tauri/Cargo.toml`: passed; three Rust tests.
- `npm run test:isolation`: passed; seed and verify reports above.
- `npm run tauri build`: passed; optimized Windows x64 executable and unsigned MSI/NSIS packages produced under `src-tauri/target/release/bundle/`. Installer installation/upgrade tests were not performed.

These checks validate the current compatibility probe and diagnostic harness, not an implemented Phase 1 account manager. The release probe was reopened for user-held device testing.

## Repeat the native fixture gate

Close the development app first to avoid locking the executable. From Git Bash or PowerShell:

```sh
npm run test:isolation
```

This builds a debug executable with bundled assets, generates a unique test run ID, and launches seed/verify processes. It writes only synthetic markers to separate diagnostic profiles, never to the probe's WhatsApp profiles. The native harness is compiled out of release builds. Test data is retained under `diagnostic-runs` for review; no automatic recursive deletion occurs.

## Required user-held device test

1. Run `npm run tauri dev`, click **Open WhatsApp Web**, and link a different number to each panel using your phones. Do not send screenshots, QR codes, cookies or credentials to the development tools.
2. Check the account identity yourself in each panel. Each must show the intended different account.
3. Quit the entire app, then restart it and reopen WhatsApp Web. Confirm that all three remain logged into their respective accounts.
4. Report whether all three passed. If any failed, record only the panel number and failure category (logged out, wrong account, provider rejection); do not include private account content.

After this gate passes, implement the production account manager and sidebar while preserving the probe as an opt-in diagnostic. Background delivery, safe downloads, origin-scoped permissions, notification delivery and unread signals need their own explicit acceptance evidence; none should be fabricated.
