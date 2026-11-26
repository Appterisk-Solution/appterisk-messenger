# Experimental macOS account support

Date: 2026-11-26 (Asia/Kuala_Lumpur). Source baseline: `0f7adb1` plus local macOS changes. App version: 0.5.1. Host: macOS 27.2.1, Apple Silicon arm64. The minimum is macOS 14; macOS 14 itself and Intel hardware have not been tested.

The implementation is versioned 0.5.2 for commit and CI packaging. The local native acceptance and DMG results below were collected before the version bump, using 0.5.1.

Follow-up in 0.5.2: the default macOS WKWebView user agent omits Safari version tokens, and WhatsApp showed its unsupported-Safari screen. WhatsApp views now start on an empty local asset, read only the native browser identity and installed Safari version metadata, then configure `Version/<installed Safari version> Safari/<native WebKit token> AppteriskMessenger/<app version>` before provider navigation. The native engine/platform tokens remain intact. This is an explicit compatibility identity; Safari metadata is not a guarantee of WKWebView feature parity. No browser data is reset and other providers keep their default identity.

An optional live, unlinked login-screen smoke test passed with `loginPrompt: true` and `unsupported: false`; evidence run `3e295455-4346-4872-bb1d-51b93e218734/provider-compatibility.json`. This test collects only browser identity and page-state booleans, never QR data or authentication values. Set `APPTERISK_MACOS_PROVIDER_SMOKE=1` when running `npm run test:macos` to opt in to this external-network smoke check; it is not a CI prerequisite or proof of real login persistence.

Final 0.5.2 validation: 27 Rust tests, 23 frontend tests, TypeScript/ESLint, native alignment at three sizes and three full storage restarts passed. The login-screen smoke check was repeated with a three-second stability delay in run `594c97a2-1209-4b5a-acf1-71e00e16aab3`; the unsupported-browser flag remained false. The rebuilt 0.5.2 arm64 DMG passed checksum and ad-hoc app signature verification. Notarization and real linked-login acceptance remain pending.

Owner follow-up: the installed 0.5.2 executable exactly matched the corrected DMG, but the Personal account still showed an immediate Safari compatibility warning. The same failure was not reproduced in an older disposable store, which reached a real QR canvas after 30 seconds. Therefore the earlier smoke pass did not establish compatibility for the owner's existing account.

Version 0.5.3 moves compatibility identity configuration into `WebviewBuilder::user_agent`, before any account navigation, and verifies `WKWebView.customUserAgent` afterward. The base identity is read only from the trusted shell; the temporary local account startup page and its navigation exceptions are removed. This eliminates the late identity-change window; it does not prove the root cause of the owner's profile-specific failure. Native fixtures now verify compatibility tokens in the first HTTP request and the optional live check requires a QR canvas after 30 seconds. No owner profile data, cache, service worker or Keychain item is cleared.

Installed 0.5.3 follow-up: the native identity was verified in the actual Personal account, but its old Safari warning initially remained. Clicking the existing Reload account control while the app was foregrounded restored the QR login screen at `https://web.whatsapp.com/login/`. The same installed Personal account was then visually/accessibility-verified after three complete Command-Q quits and reopens; all three returned to the QR screen without the Safari warning. However, the owner then scanned the QR code and reported that the Safari warning returned and linking did not complete on the phone. Therefore WhatsApp account linking is FAIL on this host; the login-screen checks are insufficient to claim provider compatibility. No Keychain items were deleted or access rules edited, and no browser storage was cleared. A temporary unlinked comparison sidebar entry was removed using the normal profile-retaining Remove account action.

0.5.3 checks: 26 Rust tests, 23 frontend tests, TypeScript/ESLint, formatting, native first-request identity checks, alignment and three synthetic storage restarts passed (run `0ebca464-f042-459a-8f71-0f6b5ee30748`). Its optional live check also reached a QR canvas after 30 seconds. A separately identified ad-hoc hardened bundle of the diagnostic executable reached the same QR state (run `a84d9d29-e329-43ed-af43-0bbad3615b6c`); no JIT/security entitlement exceptions were added. The arm64 0.5.3 app and DMG passed signature/checksum verification and were installed locally. Real linked-account authentication and its restart persistence remain owner-dependent.

The Windows-only account initialization guard has been replaced with a native macOS adapter. The pinned Tauri 2.12.7 / Wry 0.57.3 implementation was inspected: named persistent WKWebView stores use `WKWebsiteDataStore::dataStoreForIdentifier` on macOS 14+, while older systems silently fall back to the shared default store. The adapter rejects older systems before creating a view. Each existing account UUID maps directly to the native store UUID; the live WKWebView's persistent flag and identifier are checked before showing it. Metadata IDs, profile locks and removal-without-storage-deletion are preserved. Windows profile APIs remain platform-gated.

## Actual local results

- PASS: 25 Rust tests, including stable/distinct store identifiers and rejection of remote/lookalike shell origins.
- PASS: 23 frontend tests, TypeScript production build and ESLint.
- PASS: three independent named native stores with identical storage keys and different synthetic values, seeded once then verified after three complete application process exits and restarts. Checks cover cookies, localStorage, IndexedDB, stored blobs, Cache Storage and existing service-worker registrations.
- PASS: synthetic remote pages cannot invoke the account snapshot command.
- PASS: production account creation/switching runs with a disposable unlinked WhatsApp profile under the diagnostic run root; selecting the same account again does not create another WebView. Modal hide/restore calls succeed. This establishes native initialization, not provider login or visual acceptance.
- PASS: local arm64 release app/DMG packaging, ad-hoc app signature verification and DMG checksum verification. No Apple notarization is claimed. Rust compilation still reports unused Windows integration code warnings on macOS.
- PASS: account frames account for the shell WKWebView safe-area inset (32 points on this host). DOM content rectangles are converted through the shell's native coordinate space before positioning child views. Native frame assertions pass at 1280×860, 1100×760 and 1600×1000, including header/footer clearance after resizing. Regression evidence: `e01563d4-5cb1-4a2e-9075-3c7576af3bfb/geometry.json` in the same local diagnostic-runs directory.
- Evidence run: `b406a182-8e10-4504-afd8-ffa79c94c83b`, retained locally under `~/Library/Application Support/com.appterisk.messenger/diagnostic-runs/<run>/phase-{0,1,2,3}.json`. Only synthetic check results are recorded; no real authentication storage is inspected.

Run `npm run test:macos` to repeat using new disposable stores. The fixture is debug-only and uses a loopback HTTP origin. It never links an account or touches the owner's metadata database. Test profiles are retained.

## Availability and remaining acceptance

macOS automatic updates are disabled before plugin initialization and command execution, so Windows feeds/installers are not used. Mac packaging defaults to app/DMG, macOS 14 minimum and no updater artifacts. CI builds an ad-hoc-signed Apple Silicon DMG and runs the native fixture. CI has not yet been run for these changes.

Native notifications, managed downloads, media permission controls, launch at login, activity-based automatic locking and account shortcuts while provider content has focus are unavailable. Media requests and downloads are denied; unavailable settings are labeled/disabled. Manual locking remains available. Shell shortcuts accept Command as well as Control. Existing tray, external popup routing and normal Tauri application shutdown are retained; broader desktop integration needs manual testing.

NOT TESTED: real WhatsApp, Telegram or Discord authentication; three-restart real-login persistence; provider messages/media; notification delivery; visual resizing/modal behavior; tray and external-link acceptance; Apple Developer ID signing/notarization; Windows regressions on a Windows host. A synthetic pass does not establish provider compatibility.

Owner acceptance: quit the previous app normally, install the new experimental DMG, open an account and link through the official provider website. After linking, quit the whole app with Command-Q and reopen three times to verify login persistence. Do not copy Windows browser profiles. Existing profiles and account metadata have not been deleted or migrated.

## 0.5.4 owner linking acceptance

The WhatsApp account identity now uses the native WebKit/platform tokens plus the installed Safari version in the standard Safari shape, without an Appterisk product suffix. It is configured before view creation and verified natively. No provider feature APIs are replaced. Optional `--macos-compatibility-log` diagnostics collect only typed capability booleans and fixed error categories; normal launches do not enable them.

Owner acceptance on 2026-11-26: after scanning the Personal account QR in the installed 0.5.4 diagnostic build, the owner explicitly reported “Chats opened and Mac is linked”. Therefore actual WhatsApp linking is PASS on this host in 0.5.4, superseding the failed 0.5.3 result above. Chat contents were not inspected or recorded. Persistence of this real linked session across three complete restarts is still pending; Telegram/Discord authentication and other platforms remain untested by this follow-up.

The native fixture passed first-request identity, geometry and three synthetic store restarts, including nonextractable AES-GCM key persistence/decryption (run `bde9fba3-3f6f-453a-bfe5-f20d6d318697`). The arm64 0.5.4 app and DMG were rebuilt and verified with ad-hoc signature/checksum checks. No notarization is claimed.
