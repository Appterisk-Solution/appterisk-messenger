> Current 0.2.0 status: The user confirmed three distinct 0.1.2 accounts passed three restarts before production implementation. The account manager is now implemented; see [0.2.0 release verification](RELEASE_0.2.0.md). Earlier pending statements below are historical. The user also separately confirmed all three adopted accounts stayed logged in through three restarts on the installed 0.2.0 manager.

> Current status (0.1.2, 2026-11-24): The Windows browser-profile path representation was fixed. The user confirmed login persistence through three restarts in Edge, native WebView2, minimal Tauri and the main application. Earlier pending/failure entries below are historical. Three distinct phone accounts have not been confirmed. Production UI and SQLite remain outside this fix. See [the release investigation](AUTH_PERSISTENCE_0.1.2.md).

# Compatibility gate — not passed

No platform is currently supported for production. No real WhatsApp sessions were linked or inspected. Windows native compilation and app launch now succeed after the local .NET native-image repair and MSVC/SDK installation documented in BUILD_SETUP.md. Three native panels were observed loading the official provider login page. Storage isolation and real linked-session persistence remain pending.

## API evidence and decisions

The selected Tauri crate is pinned to 2.12.7, with `unstable` required for `WebviewBuilder`/child WebViews. See the [official builder API](https://docs.rs/tauri/2.12.7/tauri/webview/struct.WebviewBuilder.html), [Webview API](https://docs.rs/tauri/2.12.7/tauri/webview/struct.Webview.html), and [capabilities documentation](https://v2.tauri.app/security/capabilities/).

Windows: three separate `data_directory` values under the OS local app-data root; never incognito. Async commands avoid the documented synchronous WebView creation deadlock. Native runtime validation is pending.

macOS: directory-based isolation is not supported by WKWebView. `data_store_identifier` requires macOS 14+. The adapter is intentionally disabled until native availability checks, persistent store deletion and isolation have been tested on ARM64/x64. Older macOS is unsupported.

Linux: Tauri exposes data directories, but independent WebKitGTK contexts, cookies and storage still require native verification. Disabled pending that verification. Mobile is outside scope.

## Reproducible gate

1. Install prerequisites in README. Run `npm install`, `npm run tauri dev` on Windows 11. Record OS, WebView2 runtime, Rust, app, Tauri and resolved wry versions (`cargo tree -i wry`).
2. Click **Test local isolation**. Save `ALPHA`, `BRAVO`, `CHARLIE` in panels 1–3. Each panel must display only its own localStorage and persistent cookie. Reload all panels; confirm markers. Quit the entire process, restart and reopen the fixture; confirm again.
3. Press **Attempt forbidden IPC** in each panel. All must report DENIED. This fixture runs in the same permission-free child labels as remote pages. Also verify rejection from a remote page using a separate instrumented test build; do not enable production remote debugging or inspect private account data. Failure is a release blocker.
4. Open WhatsApp Web and link three distinct numbers using phones. Confirm correct independent accounts. Reopen/restart and verify all sessions persist. Reopening the probe intentionally recreates views; ordinary MVP account switching is not implemented.
5. Resize, maximize, move between monitors of different DPI, hide/show panels and open/close the guide. No native view may cover the guide or toolbar. Verify keyboard navigation and Escape.
6. Manually send/receive a harmless test message and upload an attachment in each linked session. Test offline recovery, provider revocation and browser rejection. No compatibility spoofing is used.
7. Record downloads, calls and notifications as **restricted in probe**, not passed. Permissions and downloads are denied. Safe confirmed downloads and origin-scoped permission controls must be implemented and tested before claiming those MVP features.
8. Record idle and three-profile private working-set measurements and test duration. No memory advantage is claimed.

| Gate | Windows 11 | macOS | Linux |
| --- | --- | --- | --- |
| Native compile/build | Debug/release builds and MSI/NSIS packaging passed; install test pending | Pending | Pending |
| Three persistent profiles | Pending | Adapter disabled | Adapter disabled |
| Local storage/cookie isolation | Passed with IndexedDB across two fresh processes; see PHASE1_GATE.md | Pending | Pending |
| Three QR-linked sessions and restart | Failed on 0.1.0; 0.1.1 user verification pending | Pending | Pending |
| Remote IPC denial | Local child fixture rejected IPC; actual remote-origin test pending | Pending | Pending |
| DPI / overlays / keyboard | Pending | Pending | Pending |
| Upload / messages / offline / revoke | Pending | Pending | Pending |
| Download / device permissions / notifications | Restricted, not validated | Pending | Pending |
| Installer / performance | Pending | Pending | Pending |

If the provider rejects WebView2, record that result as a blocker. Do not substitute Electron, spoof features, share profiles or mark the gate passed.
