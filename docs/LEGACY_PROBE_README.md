> Current status (0.1.2, 2026-11-24): The Windows browser-profile path representation was fixed. The user confirmed login persistence through three restarts in Edge, native WebView2, minimal Tauri and the main application. Earlier pending/failure entries below are historical. Three distinct phone accounts have not been confirmed. Production UI and SQLite remain outside this fix. See [the release investigation](docs/AUTH_PERSISTENCE_0.1.2.md).

# Appterisk Messenger

Independent, unofficial desktop wrapper for the genuine WhatsApp Web interface. Not affiliated with WhatsApp, Meta or Altus. Provider terms and linked-device restrictions apply; compatibility is not guaranteed.

## Status: compatibility prototype, not MVP

The specification requires profile isolation to be proven before Phase 1. This repository implements the first gate's source: a Vue/strict-TypeScript shell controlling three Tauri child WebViews with stable separate WebView2 data directories, reload, hide/show, resize and a guide that hides native content before opening. A local fixture tests localStorage/cookie persistence and forbidden IPC without credentials.

**Full compatibility is not yet verified.** Windows native compilation and application launch now succeed after repairing the local 32-bit .NET native-image cache and installing MSVC tools/Windows SDK. Three native panels load WhatsApp's login page. Independent linked sessions and restart persistence remain unverified. Account CRUD, SQLite, settings/themes and remaining Phase 1 features are pending the compatibility gate. No later phases are implemented.

See [plan](docs/IMPLEMENTATION_PLAN.md), [architecture](docs/ARCHITECTURE.md), [manual compatibility gate](docs/COMPATIBILITY.md) and [execution report](docs/EXECUTION_REPORT.md).

The credential-free three-profile cookie/localStorage/IndexedDB restart diagnostic passes, including native browser shutdown acknowledgments across three restart cycles. The real WhatsApp login test failed on 0.1.0; 0.1.1 adds coordinated shutdown, effective-profile verification and metadata-only diagnostics and requires a real-session retest. See the [persistence investigation](docs/SESSION_PERSISTENCE.md) and [Phase 1 prerequisite record](docs/PHASE1_GATE.md). Repeat local checks with `npm run test:isolation`; native Cargo commands can use `npm run cargo -- ...` to initialize the Windows build environment from Git Bash automatically.

## Prerequisites

Windows 11, Edge WebView2 Evergreen runtime, Node 24+, npm, Rust stable MSVC with rustfmt/Clippy, and Visual Studio Build Tools with Desktop development with C++, MSVC x64/x86 tools and Windows SDK. Use Developer PowerShell for native commands. A coreutils link.exe is not the Microsoft linker.

See the [official Tauri prerequisites](https://v2.tauri.app/start/prerequisites/). macOS later needs Xcode command-line tools and a macOS 14+ persistent-store adapter; Linux later needs WebKitGTK 4.1/GTK development packages. Both adapters are currently disabled pending validation. Signing/notarization are not configured; CI bundles are unsigned experimental artifacts.

## Development and build

```powershell
npm install
npm run tauri dev
npm run tauri build
```

Use `npm run dev` to preview only the HTML shell. Browser preview cannot create native panels and reports that restriction. Dependencies are pinned by npm and Cargo lockfiles. The Tauri crate is 2.12.7, using its unstable child-WebView API.

`npm run tauri dev` works from Git Bash or PowerShell: the project launcher adds the installed Cargo directory to its own PATH and loads the Microsoft C++ environment using vswhere/VsDevCmd. If C++ tools are missing it stops with one prerequisite message. It does not change your shell configuration.

```powershell
npm run typecheck
npm run lint
npm test
npm run build
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
```

CI checks the frontend and builds on Windows. Passing CI does not prove WhatsApp support. The manual gate remains mandatory. The diagnostic UI uses plain CSS; Tailwind, shadcn-vue, Pinia and Lucide are deferred to the Phase 1 shell.

## Privacy and limitations

No remote backend, telemetry, message database, AI, protocol reimplementation or automated login. Browser-managed authentication remains in separate local directories; data is not claimed encrypted. No QR codes, credentials, messages or cookies are intercepted.

Profiles live under the OS app-local-data directory for `com.appterisk.messenger`, in `compatibility-profiles/<fixed-UUID>`. Reopening either test recreates views and preserves profiles. Quit before manually removing this specific directory to reset the probe. There is no account-removal feature; uninstalling may retain browser data.

Main-frame navigation allows only the exact HTTPS provider origin. Popups, downloads and permission requests are denied in this diagnostic build. No external link opener is granted. Camera, microphone, notifications and downloads are not claimed supported. Uploads and ordinary messaging still require native/provider testing. There are no fabricated unread counts or online states.
