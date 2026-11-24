> Current 0.2.0 status: The user confirmed three distinct 0.1.2 accounts passed three restarts before production implementation. The account manager is now implemented; see [0.2.0 release verification](RELEASE_0.2.0.md). Earlier pending statements below are historical. The user also separately confirmed all three adopted accounts stayed logged in through three restarts on the installed 0.2.0 manager.

# Authentication persistence — 0.1.2

## Result and root cause

The user confirmed that WhatsApp login survived three full restarts independently in Microsoft Edge, the standalone native Rust WebView2 control, the minimal Tauri control, and the patched main Appterisk application. This establishes persistence for the tested account in each configuration on this Windows machine; it does not establish simultaneous acceptance with three different phone accounts.

The reproducible fault was the Windows verbatim profile path passed to WebView2. Rust canonicalization produced `\\?\C:\...`; the application supplied this representation directly as the browser data directory. In two independent disposable-profile A/B regressions, this representation lost a persistent cookie at the first cold restart while localStorage, IndexedDB records, a large Blob and a synthetic non-extractable CryptoKey remained usable. Ordinary absolute paths retained all persistent fixtures through three restarts. This explains why the previous narrower storage checks and graceful-shutdown changes did not resolve the actual failure. The lower-level Chromium implementation mechanism has not been established.

`src-tauri/src/profile_paths.rs` now converts the browser-facing path to its ordinary absolute Windows representation with `dunce::simplified`. Canonical paths remain in use for confinement checks. Unrepresentable paths fail closed. The physical profile directories and fixed identifiers are unchanged: no profiles or authentication data were copied, migrated, deleted or reset. The isolation fixture now exercises the same canonicalization and normalization boundary as production.

## User-confirmed matrix — 2026-11-24

| Configuration | Independent profile | Three restarts |
| --- | --- | --- |
| Microsoft Edge (`npm run test:edge`) | Edge comparison directory | Passed, user confirmed |
| Native Rust WebView2 (`npm run test:native`) | Native control directory | Passed, user confirmed |
| Minimal Tauri (`npm run test:tauri`) | Minimal Tauri control directory | Passed, user confirmed |
| Appterisk 0.1.2 release executable | Existing account profile | Passed, user confirmed |

Versions 0.1.0 and 0.1.1 failed the real-user test. Production UI and SQLite work were not undertaken as part of this fix.

## Controls and comparison

The native diagnostic uses `webview2-com`, one Win32 window and one standard controller, default environment options and user agent, and no Tauri, Wry, JavaScript injection or IPC. Its dependency tree was checked for Tauri/Wry. The minimal Tauri diagnostic has one remote WebView, no application scripts, plugins, privileged command handlers or remote capability grants. Stock Tauri internal initialization remains a disclosed difference. Each has a fixed separate profile, an exclusive process-held profile lock, resize handling and normal controller shutdown with BrowserProcessExited acknowledgment. Smoke runs verify navigation and shutdown only; authentication results above come from the user.

The public reference [whatsapp-web.view](https://github.com/Adytm404/whatsapp-web.view/tree/ee6ad2bb46aa740c94dd9513070c3dc19a5b6ac5) was reviewed at commit `ee6ad2bb46aa740c94dd9513070c3dc19a5b6ac5`. No license was declared and no source was copied. It uses a stable ordinary AppData profile path, default native environment options and a standard controller, plus a named mutex. Its identity spoofing, notification shim, automatic permission approvals and page bridges were not adopted. Its close path did not provide the same explicit browser-process exit acknowledgment. Its own persistence claims were not independently tested.

## Storage and lifecycle investigation

Diagnostics collect metadata only: executable/version, runtime/default user agent, profile identifier, absolute/effective paths, existence, private-mode state, numeric failures, storage counts and aggregate quota. Actual cookie values, authentication tokens, keys and messages were not read or exported. File integrity inspection used counts, byte totals, zero-length counts and directory permission metadata, not database contents. No WebView2 environment or policy override was found in the inspected locations. No security exclusions or browser settings were changed; absence of logged security events cannot rule out interference.

The original profile configuration was persistent and non-private. Controller closure now retains environment references until normal browser exit; timeout leaves the application open rather than killing browsers. The Chrome_WidgetWin_0 warning 1412 was observed after normal browser exits and was not established as the persistence cause. Session cookies are expected to expire; no real cookie lifetime was altered. Service-worker/cache availability and storage quota are inspected through non-sensitive metadata, not by reconstructing WhatsApp credentials.

## Regression and release evidence

- TypeScript checks, ESLint and three frontend tests passed.
- Rust formatting, main and control-workspace Clippy with warnings denied, and six main Rust tests passed.
- Expanded three-profile isolation passed seed plus three cold restart processes, covering persistent cookies, localStorage, IndexedDB, Blob integrity, synthetic non-extractable key use, expected session-cookie expiry, privileged IPC rejection and normal browser exits. Passing run: `55ea4b15-d03e-4afb-8f6e-06fc9db9e428`.
- Verbatim-path reproductions failed persistent-cookie retention in runs `c8ac72f7-7cfd-4a3d-997e-b2993a28d802` and `6b1080bf-a3db-4827-8cf6-2461fa773676`. These use disposable synthetic profiles only.
- Both release controls built and passed navigation/shutdown smoke runs. The main production build passed.

Artifacts are `src-tauri/target/release/appterisk-messenger.exe`, `src-tauri/target/release/bundle/nsis/Appterisk Messenger_0.1.2_x64-setup.exe`, and `src-tauri/target/release/bundle/msi/Appterisk Messenger_0.1.2_x64_en-US.msi`. Installers are unsigned; installer upgrade behavior was not part of the authenticated restart matrix.
