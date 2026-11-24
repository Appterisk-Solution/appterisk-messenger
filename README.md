# Appterisk Messenger 0.2.0

A Windows desktop account manager for the official WhatsApp Web interface, built with Rust, Tauri 2, WebView2 Evergreen, Vue 3, TypeScript, Tailwind CSS, Pinia and SQLite. There is no external backend. Chat content is rendered by WhatsApp in native WebViews, not recreated in Vue.

The user confirmed three distinct accounts retained independent logins through three restarts on the 0.1.2 baseline before implementation began, then separately confirmed all three adopted accounts remained logged in through three restarts in the installed 0.2.0 manager. The browser-path fix and graceful shutdown are preserved. Automated fixtures are recorded separately from these real-user results. See [release status](docs/RELEASE_0.2.0.md).

## Use the account manager

Open `src-tauri/target/release/appterisk-messenger.exe`, or install the unsigned Windows x64 NSIS/MSI package under `src-tauri/target/release/bundle/`.

1. Click **Add account**. Give a new account a name, or explicitly choose **Adopt Existing account 1/2/3** to reuse a compatibility profile in place.
2. New accounts are linked inside the official WhatsApp interface. Existing profile adoption does not copy, move or reset browser data.
3. Click sidebar icons to switch. Right-click an icon or use **Account options** to rename, customize, reload, reorder or remove it.
4. Drag icons to reorder, or use the accessible up/down controls in **Settings → Accounts**.
5. **Settings → General** offers light/dark/system themes, a startup account override and saved language preferences. Without an override, the last selected account is restored. The current shell is English; WhatsApp controls its own language.

Removal requires confirmation and removes metadata only; the on-disk browser profile is retained. It does not log a device out of WhatsApp. Use WhatsApp's Linked devices on the phone to unlink. Removed compatibility profiles can be explicitly adopted again. Removed new-account profile directories are retained without an automatic reimport UI.

`Ctrl+1` through `Ctrl+9` select accounts, `Ctrl+R` reloads, and `Ctrl+,` opens settings. Native account views forward these accelerators while focused. Closing with X closes controllers and waits for BrowserProcessExited rather than force-terminating them.

## Data and security

Application data lives under `%LOCALAPPDATA%\com.appterisk.messenger`:

- `appterisk.sqlite3`: accounts, application settings and schema migration metadata only.
- `accounts/<permanent UUID>`: new persistent browser profiles.
- `compatibility-profiles/<original UUID>`: existing profiles reused in place after explicit adoption.
- `logs/persistence.jsonl`: non-sensitive lifecycle/runtime/path metadata.
- `diagnostic-runs/<run UUID>`: retained synthetic test profiles and reports.

Browser-facing Windows paths are ordinary absolute paths normalized with `dunce::simplified`; canonical paths are used for confinement checks. Incognito is explicitly disabled. Profiles and IDs are never regenerated on startup. No browser authentication databases are copied or migrated. An exclusive application lock and per-account locks prevent concurrent new-version instances from opening the same profiles. Close older Appterisk versions before use or upgrade.

Only the trusted local shell has application command capabilities. Remote children have no capability grants, and every privileged handler separately verifies the caller label and origin. Main-frame navigation stays on the exact official HTTPS origin; valid HTTP(S) popup links open in the system browser. New sensitive permission requests and downloads are denied in this release. No identity spoofing, TLS relaxation or sandbox bypass is used.

Notification preferences are saved metadata; notification delivery is not implemented or verified. Unread counts, background messages, media/calls/download support, macOS and Linux are unverified. No unread badges are fabricated. Installer upgrade evidence distinguishes filesystem preservation from user-confirmed authentication continuity.

The NSIS upgrade from 0.1.2 to 0.2.0 passed with all 2,228 existing profile files unchanged in count, size and modification timestamp. The final installed payload was verified separately, followed by user-confirmed three-account login persistence across three restarts. MSI upgrade remains untested.

## Build and checks

Windows needs Node/npm, Rust stable MSVC, Visual Studio C++ Build Tools and Windows SDK, and WebView2 Evergreen. The wrappers initialize Cargo/MSVC for Git Bash or PowerShell. See [build setup](docs/BUILD_SETUP.md).

```sh
npm ci
npm run tauri dev
npm run typecheck
npm run lint
npm test
npm run cargo -- fmt --manifest-path src-tauri/Cargo.toml --check
npm run cargo -- clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
npm run cargo -- test --manifest-path src-tauri/Cargo.toml
npm run test:isolation
npm run test:manager
npm run tauri build
```

Close a running development executable before rebuilding it on Windows. `test:isolation` remains mandatory: three disposable profiles, durable cookies, localStorage, IndexedDB, Blob/non-extractable-key checks, session-cookie expiry, IPC rejection and three cold restarts. `test:manager` adds real native switching/reuse, modal visibility, resizing, metadata restart/adoption/removal, actual remote-origin IPC rejection and normal shutdown acknowledgments. It opens the public WhatsApp origin in disposable, unlinked profiles; no authentication acceptance is inferred.

## Optional diagnostics

Use **Settings → Diagnostics → Restart in diagnostic mode**, or `npm run dev:diagnostics`, or launch the executable with `--diagnostics`. The original three-panel compatibility probe is preserved and can return to the account manager through an orderly restart. Do not open it concurrently with an older app using the same profiles.

The independent 0.1.2 controls remain available via `npm run build:controls`, `npm run test:native`, `npm run test:tauri` and `npm run test:edge`. These use separate profile directories. See the [persistence root-cause report](docs/AUTH_PERSISTENCE_0.1.2.md), [architecture](docs/ARCHITECTURE.md), [changelog](CHANGELOG.md) and [current implementation status](docs/IMPLEMENTATION_PLAN.md). Historical probe documentation is retained under `docs/LEGACY_*`.

Independent and unofficial; not affiliated with WhatsApp or Meta.
