# Appterisk Messenger 0.5.0 - experimental updater

The Rust-side updater and **Settings > Updates** are implemented. See [release verification](RELEASE_VERIFICATION_0.5.0.md), [GitHub feeds](UPDATE_SERVER.md), [release guide](RELEASE_GUIDE.md) and [signing/recovery](CODE_SIGNING.md). Tauri signatures and Windows publisher signing are separate; Authenticode is currently unavailable. The isolated two-version installed-updater test passed; real-account post-upgrade acceptance remains pending. Stable publication is gated; Beta is for explicitly opted-in testing. Existing messaging implementations remain limited to WhatsApp, experimental Telegram and experimental Discord.

## Prior v0.4.0 implementation

A Windows account manager displaying official WhatsApp Web and experimental Telegram WebK/Discord through independent native WebView2 profiles. Rust, Tauri 2, Vue 3, TypeScript, Tailwind CSS, Pinia and SQLite; no Electron, CEF, unofficial WhatsApp API or external backend.

The user-verified v0.3.0 baseline is tagged `v0.3.0-stable`. All v0.3.0 manual acceptance results, including three accounts through three full restarts, are user-confirmed. Development continues on `feature/multi-platform`.

This is an incremental v0.4.0 validation stage, not the completed multi-service release. Add account offers WhatsApp, **experimental Telegram** and **experimental Discord**. Other proposed services are visibly unavailable until their sequential implementation and real provider validation. Telegram plus the three WhatsApp accounts passed the three-restart login checkpoint by user confirmation; other Telegram features remain unverified. Telegram uses the official [WebK client](https://telegram.org/apps), normal provider authentication, a new empty UUID profile and default browser identity. No provider compatibility is inferred from synthetic tests. See [platform compatibility](docs/PLATFORM_COMPATIBILITY.md) and [current verification](docs/RELEASE_0.4.0.md).

Schema v3 preserves account IDs, original profile keys, settings, ordering and permission decisions. Browser profiles are never moved or copied. **Schema downgrade to v0.3.0 is refused after first v0.4.0 launch.** Keep the stable source checkpoint for reference, not as an executable downgrade against migrated metadata.

## Install and accounts

The installed executable is `%LOCALAPPDATA%\Appterisk Messenger\appterisk-messenger.exe`. Release artifacts:

- `src-tauri/target/release/appterisk-messenger.exe`
- `src-tauri/target/release/bundle/nsis/Appterisk Messenger_0.4.0_x64-setup.exe`
- `src-tauri/target/release/bundle/msi/Appterisk Messenger_0.4.0_x64_en-US.msi`

Installers are unsigned. The staged 0.3.0 to 0.4.0 NSIS upgrade passed with 3,303 existing profile files unchanged during installation; metadata comparison also passed after migration. Telegram and all three WhatsApp accounts passed real post-upgrade login persistence across three full restarts by user confirmation. Discord acceptance remains pending. See [0.4.0 verification](docs/RELEASE_0.4.0.md). The NSIS upgrade from 0.2.0 passed with all 2,995 existing profile files unchanged in count, size and modification time during installation. The installed payload matches the final NSIS package. This is filesystem preservation evidence, not a claim that manual authentication or notification tests passed. MSI upgrade, signing, automatic updates and other operating systems are not claimed verified.

Add and name accounts, or explicitly adopt an existing compatibility profile in place. Adoption never copies or migrates browser authentication databases. Click sidebar icons to switch; right-click or use Account options to rename, customize, reload, reorder or remove. Drag icons or use Settings → Accounts up/down controls to reorder. Removal requires confirmation and retains browser data; unlink through WhatsApp Linked devices on the phone if desired. Original compatibility profiles can be adopted again. Account count has no artificial software limit.

## Notifications and unread indicators

1. Enable each account under **Settings → Notifications**. Existing per-account preferences are preserved, including disabled notifications.
2. Under **Privacy and Security**, select that account and explicitly allow **Notifications**, or approve the origin-scoped prompt when WhatsApp requests access. Permission changes reload that account. WhatsApp's own notification settings and Windows notification settings must also permit delivery.
3. Global notifications, Do Not Disturb, sound and hidden previews are configurable. Hidden previews are on by default. Locking always hides new notification previews.

Only genuine WebView2 `NotificationReceived` events produce native Windows notifications. No test-message button, fabricated message events, JavaScript Notification replacement or private WhatsApp API is used. Toasts identify the configured account; clicking uses a validated local activation URI and selects that existing account, subject to the application lock. Payload text is bounded and XML-escaped; provider URLs, icons and actions cannot choose application commands. Duplicate native notification objects are suppressed; identical content from distinct notifications is not deduplicated as though it were the same message.

When previews are enabled, text from actual provider notifications is passed to Windows in memory; Windows may retain notification content in its own notification history. Appterisk does not write message content to SQLite or logs. Existing host toasts are cleared when locking, disabling notifications, entering DND, changing privacy mode or quitting.

Telegram and Discord currently use a generic activity dot from genuine native notifications; numeric title parsing is not claimed. WhatsApp unread numbers use narrowly recognized WhatsApp document-title signals and can represent unread chats rather than messages. Unrecognized titles do not invent a count or clear a known indicator. A real notification can set a generic dot. The tray shows a red activity dot and aggregate tooltip; `+` indicates additional unread activity without an exact count. Provider changes can invalidate these signals. Real incoming-message and read-state acceptance must be confirmed separately.

## Background and tray

All restored accounts initialize after the shell is ready and unlocked. Existing controllers are reused on switching, with only the active view visible. Inactive views stay alive; no suspend, background-security bypass or browser-identity spoof is used. WebView2 can throttle hidden pages, and WhatsApp can suppress notifications for a foreground chat. Sleep, network conditions and Windows Focus Assist also affect delivery; hidden/minimized/tray delivery requires real-message verification.

The tray menu offers Open, account selection, notifications, DND, Settings, Lock and Quit. Left click restores the window. **X genuinely exits by default.** Close-to-tray, minimize-to-tray, launch on Windows startup and start minimized are explicit settings and default off. Quit always uses controller closure and BrowserProcessExited acknowledgments; no forced browser termination is used. Instance and profile locks plus Tauri single-instance routing prevent simultaneous new-version profile access. Close older versions before use or upgrade.

## Attachments and permissions

Uploads use official WhatsApp attachment controls and WebView2's native file picker. Downloads use native WebView2 events, an optional save dialog, a configurable directory, progress and cancellation in **Settings → Downloads**. Staging files are finalized atomically without overwriting an existing destination; collisions receive numbered filenames. Filenames are sanitized for traversal and Windows reserved names. Downloads are never automatically executed, and WhatsApp receives no general filesystem IPC.

The credential-free native attachment fixture passed actual download byte-integrity and collision checks. Real image/document/video/audio upload/download, cancellation and voice-message compatibility remain separate manual checks.

Only the exact official WhatsApp HTTPS origin can request notifications, camera, microphone, clipboard read or autoplay through the host. Unknown origins and kinds are denied. A host prompt requires an explicit decision, supports remembering it for that account/origin and expires denied after one minute. Settings allow revocation; changing a permission reloads the account to stop active capture. Windows device/privacy controls remain authoritative. Voice/video calling is not claimed supported without verification.

## Local lock and languages

Configure a local password under **Privacy and Security**, then use the sidebar/tray Lock action or an inactivity timeout. The lock hides all native children, rejects account operations while locked and restores the previous account after unlocking. On restart, a configured lock starts locked. Passwords use Argon2id with OS-random salts; plaintext passwords are never stored. Unlock attempts are rate-limited.

This is a UI lock, not browser-profile encryption or protection from another Windows administrator. Browser pages and ongoing background activity can remain alive while locked. Accounts are initialized after the first unlock on a locked startup.

The production shell supports English, Simplified Chinese and Traditional Chinese, including desktop settings, permission prompts and lock controls. WhatsApp manages its own language. The original developer probe remains English. Legacy language preferences remain readable, with English fallback where a locale is unsupported.

Shortcuts work in the shell and focused native account views:

- Ctrl+Tab / Ctrl+Shift+Tab: next / previous account.
- Ctrl+1–9: select account.
- Ctrl+R: reload the active account.
- Ctrl+Comma: settings.

## Data and development

Data stays under `%LOCALAPPDATA%\com.appterisk.messenger`: application metadata in `appterisk.sqlite3`, new profiles in `accounts/<UUID>`, adopted profiles in `compatibility-profiles/<original UUID>`, metadata-only logs in `logs`, and retained synthetic fixtures in `diagnostic-runs`. SQLite schema v2 adds origin permissions; lock hashes and desktop preferences are application settings. Browser cookies, authentication tokens, encryption keys and messages are never placed in the application database.

Canonical paths enforce confinement; `dunce::simplified` supplies ordinary absolute paths to WebView2. Incognito is explicitly disabled. No startup, shutdown or upgrade browser-storage cleanup is performed. Only the local shell has command capabilities, and privileged handlers also verify caller label/origin. Remote-origin rejection is tested independently against the official WhatsApp origin.

Windows requires Node/npm, Rust stable MSVC, Visual Studio C++ tools/Windows SDK and WebView2 Evergreen. Wrappers initialize Cargo/MSVC from Git Bash or PowerShell. Close running development binaries before rebuilding.

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
npm run test:desktop
npm run tauri build
npm run test:upgrade
```

The upgrade test refuses to downgrade an existing newer install and requires all Appterisk processes closed. `--refresh` verifies the same final target version without replacing initial upgrade evidence. None of these synthetic tests establish real WhatsApp delivery or authentication.

Use **Settings → Diagnostics** or launch with `--diagnostics` after quitting to access the preserved compatibility probe. Independent 0.1.2 native/minimal-Tauri/Edge controls remain available through existing scripts. See [architecture](docs/ARCHITECTURE.md), [release evidence](docs/RELEASE_0.3.0.md), [changelog](CHANGELOG.md) and [build setup](docs/BUILD_SETUP.md).

Independent and unofficial; not affiliated with WhatsApp or Meta.

## License and code signing policy

Project-owned source is available under the [MIT License](LICENSE). Dependencies, Microsoft WebView2 and remote provider websites retain their own terms; MIT does not relicense them.

See the [Code signing policy](CODE_SIGNING.md) and [privacy policy](PRIVACY.md). SignPath Foundation declined the free application for insufficient public reputation; no Windows publisher signing is claimed. A [Microsoft Store MSIX candidate](STORE_RELEASE.md) is now being prepared. Existing installers remain Authenticode-unsigned.
