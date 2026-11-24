# Release verification — 0.3.0

Date: 2026-11-24, Asia/Kuala_Lumpur. Platform: Windows x64 with WebView2 Evergreen 155.0.4283.62 on the tested machine.

## Baseline and implementation

Before development, the complete user-verified 0.2.0 source was committed as `d401ca5`, tagged `v0.2.0-stable`. Its real three-account authentication-through-three-restarts result remains historical baseline evidence, not an automatic 0.3.0 pass.

Implemented: native WebView2 notification capture and WinRT toast routing; account/global notification preferences, DND, sound and previews; native title-based unread indicators and aggregate tray activity; initialized background account WebViews; tray/restore/selection/settings/quit; opt-in minimize/close/start-minimized/startup preferences; native downloads with dialogs/progress/cancel/no-clobber finalization; native upload picker; origin-scoped permission prompts/revocation; local salted Argon2id lock and inactivity timeout; specified native/shell shortcuts; English/Simplified/Traditional Chinese production shell; migration v2 and preserved existing profiles/account manager/diagnostic probe.

## Automated verification

- TypeScript and ESLint passed; **16 frontend tests passed**.
- Rust formatting and all-target Clippy with warnings denied passed; **19 main Rust tests passed**.
- Independent control workspace formatting, Clippy and tests passed; it contains zero unit test cases and was not used to claim new real-login results.
- Mandatory synthetic isolation passed seed and three cold restart processes across three profiles, preserving durable cookies, localStorage, IndexedDB, Blob integrity and a synthetic non-extractable key; expected session-cookie expiry and child IPC rejection remained correct. Run `c696a15b-a2b5-4a27-b2a4-358d0a7b002e`.
- Extended native manager passed seed, three restart verification cycles and removal. It covers native switching without reload, one visible view, resize, modal hiding, stable UUID/order/selection, in-place adoption, removal isolation, lock hiding, rejection of trusted account switching while locked and password unlock. Run `2e8a34d0-39c9-4096-a674-6fe2cac770ba`.
- Actual `https://web.whatsapp.com` views rejected account/probe commands and `desktop_snapshot`, `desktop_save`, `desktop_unlock`, `desktop_password`, `desktop_permission` and `desktop_permission_reply`. This is separate from local-origin fixture IPC checks.
- Native attachment fixture passed actual download byte integrity and preservation of an existing filename. Run `447a32a5-e02e-499c-94b5-88ffc8d41d1c`. The initial open-staging-handle implementation failed this test; finalization now retains a temporary path without holding the conflicting write handle.
- Pure policies cover notification routing/XML safety, duplicate-object gating/preferences, provider-scoped unread state/account association, tray close defaults and explicit-quit priority, download traversal/reserved filenames/collisions, permission authorization, shortcuts, settings migration and salted password verification/rate limiting.
- Production x64 executable, NSIS and MSI builds passed. CI retains isolation and manager fixtures and adds the native desktop fixture.

Machine-readable public-fixture evidence is under `release-evidence-0.3.0`. No fake runtime message notifications were generated to obtain these results. These tests do not verify real incoming-message delivery, real attachments, voice messages, Windows logon startup or notification clicks.

## Visual review

The computer-use skill inspected the empty shell and expanded settings in a separate, empty debug-only review root. The desktop layout and navigation rendered correctly. Chinese translations have automated coverage; a native dropdown-selection action was not completed, so native visual Chinese acceptance is not claimed. No real WhatsApp account was opened for this visual review. The review window was closed normally before upgrade.

## Upgrade and artifacts

The installed 0.2.0 application was upgraded with the 0.3.0 NSIS package: exit code 0; all **2,995 existing profile files** retained their file count, sizes and modification timestamps during installation. No browser database contents were read and no authentication data was copied. The installed executable matches the expected NSIS payload hash after Tauri's documented three-byte bundle-marker patch. See `release-evidence-0.3.0/nsis-upgrade.json`.

The installed 0.3.0 process subsequently created all three existing profiles with matching effective directories, default runtime identity and non-private mode. This confirms configuration/path reuse, not authenticated identity or notification delivery. Metadata migration preserves account keys/order/settings in automated v1→v2 tests; real upgrade/account acceptance remains a separate user check.

Artifacts:

- `src-tauri/target/release/appterisk-messenger.exe`
- `src-tauri/target/release/bundle/nsis/Appterisk Messenger_0.3.0_x64-setup.exe`
- `src-tauri/target/release/bundle/msi/Appterisk Messenger_0.3.0_x64_en-US.msi`

Installers are unsigned. MSI upgrade, signing, automatic updates and cross-platform support are not claimed implemented/tested. Downgrading the v2 metadata schema into the old 0.2.0 binary is not supported by its schema guard; the stable Git checkpoint does not imply automatic data rollback.

## Manual acceptance

At the time of this record, these are **pending user confirmation** on installed 0.3.0:

| Test | Status |
| --- | --- |
| Real incoming message while account is active | Pending |
| Real incoming message while account is hidden | Pending |
| Real incoming message while app is minimized | Pending |
| Real incoming message while app is in the system tray | Pending |
| Notification shows correct account name | Pending |
| Notification click selects originating account | Pending |
| Unread indicators update on arrival and reading | Pending |
| Image/document/video/audio uploads and downloads, cancellation | Pending |
| Microphone and voice messages | Pending |
| Manual app lock hides native views and unlock restores account | Pending user check; native fixture passed |
| Three distinct accounts remain authenticated through three full restarts | Pending on 0.3.0; 0.2.0 baseline passed |
| Real accounts/authentication survive the 0.2.0 upgrade | Pending; installer filesystem preservation passed |

Enable each account's notification preference and explicitly allow notification permission for that account/origin. WhatsApp's own notification preferences and Windows/Focus Assist must permit delivery. Test actual messages from a phone, not generated developer events. Use tray Quit for full-restart tests if close-to-tray is enabled. Configure any lock password privately; do not share passwords, QR codes, messages or private attachments with development tools.

## Known limitations and references

- Only notifications WhatsApp actually emits through the native browser notification mechanism are routed. Foreground-chat suppression and provider/Windows policy can prevent a toast. Hidden controllers are alive, but background delivery is not guaranteed by that fact.
- Title counts are provider-dependent and may count chats. A generic dot/aggregate `+` is used when a count is unavailable; no exact message count is fabricated.
- Privacy mode avoids reading provider title/body; enabled previews go to Windows and may remain in OS notification history. Messages are not stored in app SQLite/logs. Lock is a UI barrier, not encryption; background page activity can continue.
- Native uploads remain provider/WebView2-controlled. Real formats, cancellation, camera/microphone/voice and any calling compatibility need separate manual testing. No TLS/sandbox or throttling flags were weakened.
- Native Windows dialogs use the OS language. The optional original probe remains English. Startup registration is opt-in; actual Windows-login launch is not yet manually verified.
- Dev-only `@vue/test-utils` transitive packages retain the previously documented moderate npm audit advisories; they are not bundled as runtime application code.

Native API choices were checked against primary sources: [WebView2 notification interface](https://learn.microsoft.com/en-us/microsoft-edge/webview2/reference/win32/icorewebview2_24), [handled notification lifecycle](https://learn.microsoft.com/en-us/microsoft-edge/webview2/reference/win32/icorewebview2notificationreceivedeventargs), [WebView2 visibility behavior](https://learn.microsoft.com/en-us/dotnet/api/microsoft.web.webview2.core.corewebview2controller.isvisible), [Tauri tray APIs](https://v2.tauri.app/learn/system-tray/) and [native dialog APIs](https://v2.tauri.app/plugin/dialog/). RustCrypto's installed Argon2 0.6 documentation supplies the salted PHC-hash/verification API. No third-party notification shim or unlicensed reference implementation was copied.

## User acceptance confirmation

The v0.4.0 request reports 18 of 19 manual acceptance tests passed, including notifications, attachments, voice, lock, shortcuts, languages and startup. The subsequent user reply ?test ok? confirms the remaining three-account, three-full-restart authentication test on v0.3.0. These are user-confirmed results, separate from automated evidence.
