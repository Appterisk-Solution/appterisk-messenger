> Current 0.2.0 status: The user confirmed three distinct 0.1.2 accounts passed three restarts before production implementation. The account manager is now implemented; see [0.2.0 release verification](RELEASE_0.2.0.md). Earlier pending statements below are historical. The user also separately confirmed all three adopted accounts stayed logged in through three restarts on the installed 0.2.0 manager.

> Current status (0.1.2, 2026-11-24): The Windows browser-profile path representation was fixed. The user confirmed login persistence through three restarts in Edge, native WebView2, minimal Tauri and the main application. Earlier pending/failure entries below are historical. Three distinct phone accounts have not been confirmed. Production UI and SQLite remain outside this fix. See [the release investigation](AUTH_PERSISTENCE_0.1.2.md).

# WhatsApp session persistence investigation — 0.1.1

## Status

The real-account acceptance test **failed on 0.1.0**. The user reported window-X closure, the same `npm run tauri dev` command on both launches, and the linked device still listed on the phone. Production UI and SQLite work remain blocked.

Version 0.1.1 fixes uncoordinated native-view teardown and unnecessary view recreation, and adds the diagnostics below. These are confirmed implementation gaps, but their causal relationship to the real WhatsApp logout is **not yet verified**. A successful storage fixture is not proof that WhatsApp's authenticated state is intact.

## Confirmed observations

- Three UUIDs are compile-time constants. Paths are absolute, canonicalized, beneath `%LOCALAPPDATA%/com.appterisk.messenger/compatibility-profiles/`. Existing directories are reused; no migration, reset, deletion or browsing-data cleanup is performed.
- WebView2's actual `UserDataFolder` and `ProfilePath` matched the requested paths on the existing account profiles across three measured restarts. All profiles reported InPrivate=false.
- Process, user and machine WebView2 folder/browser-argument overrides were absent. HKCU/HKLM WebView2 policy keys were absent. Future process overrides fail closed; effective-folder validation detects policy redirection without altering policy.
- Actual WebView2 runtime: 155.0.4283.62. Actual user agent was identical across the measured views/restarts; no user-agent override or capability spoofing was introduced. Installed Edge was 156.0.4314.42. Engine version differences alone do not establish the cause.
- Actual WhatsApp-origin metadata showed two cookies in every panel on each measured restart, localStorage entries, IndexedDB databases and Cache Storage. Service workers were registered/active in profiles 1 and 2 at capture time; profile 3 reported none. Counts can change during normal provider initialization and do not identify accounts or authenticate them.
- No script-error or unhandled-rejection events were counted at the captured page loads. Raw console strings, resource URLs, stacks and rejection reasons are intentionally not collected because they may contain private data.
- Window-close measurements recorded three normal BrowserProcessExited events before app exit on each of three measured cycles. These were metadata measurements, **not three successful login tests**.
- Warning Chrome_WidgetWin_0 / 1412 still occurs at final local-shell teardown after account browser processes have reported normal exit. This warning alone is not evidence of lost authentication or a renderer crash.

Local reports: `%LOCALAPPDATA%/com.appterisk.messenger/logs/persistence.jsonl` and `logs/origin-restarts.json`. No logs were uploaded.

## Changes

1. `.incognito(false)` is explicit. Every account view is checked against the engine's effective data directory and InPrivate flag before acceptance.
2. Reopening the same probe mode reuses existing views, with no navigation/reload. Changing fixture/provider modes closes controllers and waits for browser exit before creating replacements, without deleting profiles.
3. Window close and application exit are intercepted. The local shell remains alive while account controllers close. COM environment objects stay on the UI thread until BrowserProcessExited; then subscriptions are removed. The app exits only after all tracked account browser processes have exited. After a 15-second timeout it stays open with an error and permits retry; it never kills a browser process.
4. Metadata-only logs include executable path/version, profile ID/path/existence, runtime version, real UA, InPrivate, creation/reuse, permissions, native process failures, close requests and browser exits. No browser cleanup operation exists; the log records cleanup=false.
5. At provider page load and via **Capture storage metadata**, diagnostics collect only cookie count, localStorage length, IndexedDB database count, service-worker registration/active counts, Cache Storage cache count, storage-persistence boolean and generic error counts. Cookie names/values, database names/records, cache names/entries, messages, QR codes, tokens and keys are never read or logged.
6. The debug isolation test now waits for native browser exits and verifies synthetic cookies/localStorage/IndexedDB and denied IPC across three restart cycles. Its timeout no longer kills the app; a hung diagnostic is left open for normal closure.

This follows the documented [WebView2 browser-exit event](https://learn.microsoft.com/en-us/microsoft-edge/webview2/reference/win32/icorewebview2environment5) and [process lifecycle](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/process-related-events). No token export/import, auth bypass, TLS weakening or engine downgrade is used.

## Acceptance retest requiring phones

1. Use the same 0.1.1 executable for every launch. Prefer `src-tauri/target/release/appterisk-messenger.exe --resume-whatsapp`; `npm run tauri dev -- -- --resume-whatsapp` is also available, but do not edit source or stop the development terminal while linking/testing.
2. Link one number normally. Once the genuine provider interface is ready, click **Capture storage metadata**. No private data needs to be sent to the developer.
3. Close using X. Wait for the window to disappear on its own. A shutdown error means the cycle has not completed; do not use Task Manager to make it pass.
4. Reopen the exact same executable and confirm yourself that the correct account is still authenticated without another QR scan. Capture metadata again. Repeat three times.
5. Repeat with three distinct numbers and confirm each panel's identity yourself. Record only pass/fail, panel number, cycle, whether the phone still lists the linked device, and any generic provider rejection. Never share QR codes or account contents.
6. If login still fails despite matching folders and clean exits, run `npm run test:edge`. It opens Microsoft Edge with a separate, fixed, normal persistent profile. Manually link a number, close that Edge instance normally, then rerun the same command three times. This neither copies Appterisk profiles nor changes the browser's security/compatibility settings. Compare the authentication result, keeping in mind the engine versions differ. Edge login/restart verification is still pending user participation.

Do not mark the issue resolved or resume production work until the real-session acceptance tests pass. If state is still lost, use before/after metadata and normal-browser comparison to narrow provider/engine behavior; do not inspect authentication records or delete profiles to hide the failure.

## Regression commands

```sh
npm run typecheck
npm run lint
npm test
npm run cargo -- fmt --manifest-path src-tauri/Cargo.toml --check
npm run cargo -- clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
npm run cargo -- test --manifest-path src-tauri/Cargo.toml
npm run test:isolation
npm run tauri build
```

Four Rust tests and three frontend tests passed. Three restart cycles of the synthetic fixture passed in run `1d259b34-5686-46fb-aab2-a386fcd167ca`, including normal browser exit acknowledgments for all three profiles. Release-build packaging results are recorded in the execution report.
