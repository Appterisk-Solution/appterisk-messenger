# Desktop architecture — 0.3.0

The Vue/TypeScript shell uses Pinia, Tailwind and application CSS. It controls application metadata and native layout, never recreates chats or reads WhatsApp authentication storage. Rust owns SQLite, profile mapping and all desktop integration. Windows-specific modules isolate WebView2/WinRT/registry code for future platform adapters; no cross-platform compatibility is asserted.

## Metadata and persistence

`database.rs` retains the 0.2.0 accounts/settings repository and adds migration 2: `origin_permissions`, scoped by account/origin/kind with account-deletion cascade. Desktop preferences and the salted Argon2id lock hash are separate application-setting keys. A v1 database upgrades transactionally without changing account UUIDs, profile keys, order or existing preferences. Newer schemas are rejected. Migration/reopen tests verify data preservation; downgrade compatibility with v0.2.0's schema guard is not promised.

Browser data stays in the existing fixed directories. The unmodified `profile_paths.rs` retains canonical validation and ordinary API-facing paths using `dunce::simplified`. Adopted profiles are referenced in place; authentication databases are never copied, migrated, reset or inspected. SQLite contains no WhatsApp tokens, keys or messages. Password inputs are zeroized on the Rust side and cleared in the UI after use; only a PHC-format salted Argon2id hash is stored.

## Account manager and security boundary

`accounts.rs` serializes database/native operations through the existing async mutex. Each initialized account has one independent child controller. Startup warms restored accounts once the shell is unlocked; switching reuses documents, hides all other views, sizes the selected child and transfers focus. The layout gate also respects lock state, host permission prompts and native save-dialog overlays. Frontend modal opening still waits for successful native hiding before rendering the overlay.

The process-held instance lock and per-profile locks remain. Tauri's single-instance plugin adds restoration and validated notification activation routing. Debug fixtures use separate run roots and skip real tray/startup/protocol registration. An orderly mode restart waits for the previous process lock to release before installing single-instance routing in the replacement process.

Generated application-command permissions grant only `main` the required commands and event listening. Remote children receive no command, filesystem, dialog or event-emission capabilities. Every handler separately verifies the local shell label/origin; account mutations and desktop operations also enforce the lock. The actual remote WhatsApp origin is tested for rejection of account, desktop settings, password/unlock and permission commands.

Provider navigation stays on exact HTTPS WhatsApp Web without credentials or alternate ports. HTTP(S) popups can open in the system browser only while unlocked; other schemes are rejected. No TLS, sandbox, user-agent or browser-security relaxation is introduced.

## Native notification pipeline

`windows_notifications.rs` subscribes to `ICoreWebView2_24::NotificationReceived`. The event is marked handled to prevent default duplicate UI. Sender origin is checked; account identity is bound in the Rust subscription, never accepted from the payload. Per-account/global notification policy and DND run before native toast submission. Previews are private by default and always hidden while locked; private mode does not read provider title/body fields. Enabled previews are bounded, stripped of control/bidi characters and XML-escaped. No URLs, icons or actions from provider payloads control application behavior.

Only actual native notification events create Windows toast objects. Native object identity prevents duplicate handling while retained. Pending objects are bounded and retain documented ReportShown/ReportClicked/ReportClosed lifecycle hooks. Windows receives the toast through WinRT, using a per-user AppUserModelId registration. Logs record profile identifiers, submission/error status and numeric HRESULTs, never message bodies or raw payloads. Submission success is not a claim that Windows displayed a banner under every OS policy.

Toast activation uses `appterisk://notification/<account UUID>/<host ticket UUID>`. A strict parser rejects alternate hosts, credentials, extra path components, queries and fragments. Existing-process routing restores the window and selects an existing account; a locked application queues selection until unlock and defers provider click reporting. Cold-start routing validates the account against restored metadata. Activation never navigates to an external payload URL or invokes a payload-selected command. Host toasts are cleared on lock/privacy/DND changes and normal quit.

`desktop_policy.rs` contains pure gating, payload escaping, routing, unread and download policies. It does not generate runtime message events. The app does not replace the page's JavaScript Notification implementation or call unofficial provider APIs.

## Unread and background

Native title-change callbacks recognize only `WhatsApp`, `WhatsApp Web` and narrowly formatted numeric prefixes. Their numbers may represent chats, not messages. Unknown titles leave the prior signal unchanged; an actual notification can set a generic unread dot. Account IDs are bound in the native closure. The shell displays per-account indicators; the tray uses aggregate metadata in a tooltip plus an activity dot. No private title text is logged.

Hidden controllers stay alive. The app never calls TrySuspend, destroys a view merely to switch accounts, or sets experimental throttling/security flags. WebView2 visibility can affect scheduling, and the provider can suppress foreground-chat notifications. Real message tests across active, hidden, minimized and tray states are required rather than inferred from controller liveness.

## Tray, startup and shortcuts

`tray.rs` creates the native menu and refreshes account associations, notification/DND checks and unread indicators. Open/left click restores; account items route through existing-account selection. X exits by default; close-to-tray is opt-in. Explicit Quit always calls the existing graceful shutdown. Minimize-to-tray and start minimized are separate preferences. `windows_desktop.rs` writes the per-user Windows Run value only when the user changes startup preference; no startup entry is enabled automatically.

Shell shortcuts and native AcceleratorKeyPressed handling share the specified account-navigation keys. Ctrl+Tab/Shift+Tab, 1–9, R and Comma are reserved; unrelated provider keys are not intercepted. Locks and host dialogs prevent shortcuts from revealing native account views.

## Downloads, uploads and permissions

`windows_downloads.rs` uses DownloadStarting deferrals and native operation events for bytes, state and cancellation. A Rust-only dialog chooses the target when requested. Provider filename suggestions are reduced to safe leaf names. A unique temporary path in the approved directory is owned by the application, with its creation handle released before WebView2 writes. Keeping that write handle open caused a real fixture failure on Windows and was corrected. Completion atomically persists without clobbering existing files; collisions use numbered names. Temporary files are the app's own staging files, never browser profiles. No downloaded file is auto-executed. Uploads remain WebView2's native file-input picker under official WhatsApp controls.

`windows_permissions.rs` checks exact origin and supported kind, obtains a native deferral and presents a trusted-shell prompt. Unknown origins/kinds and locked requests are denied; unanswered requests expire denied. Remembered decisions are stored only as app metadata and applied with supported Profile.SetPermissionState. Revocation reloads the affected account to stop active capture. One-time decisions do not become persistent grants. Windows device controls still apply; calling/media compatibility needs real-device acceptance.

## Lock and shutdown

`app_lock.rs` uses Argon2id with OS-random salts and rate-limited failed attempts. A configured lock starts locked on every process restart. Manual/automatic lock hides all native children and moves keyboard focus to the trusted shell. The inactivity monitor samples Windows input activity while the application owns the foreground. Unlock verifies the hash before restoring native layout and pending account selection. This is not full profile encryption or protection against a Windows administrator; background page activity can remain alive.

Normal shutdown resolves pending permission/download deferrals, releases notification references, closes controllers and retains environments on their UI thread until BrowserProcessExited. Timeout leaves the application open for retry rather than killing browsers. The shell's default browser closes with final process exit. Manager regressions check normal exit acknowledgments independently of the lingering Chrome_WidgetWin_0 warning.

## Localization and verification

`i18n.ts` supplies English/Simplified/Traditional Chinese production-shell strings. UI language changes update rendering/document language and explicit tray-menu locale. Native OS dialogs follow Windows language; WhatsApp and the optional original probe manage their own UI. The source codemod only assists development; runtime translations are explicit Vue calls.

Verification separates pure/unit tests, native synthetic fixtures, visual review, installer file preservation and user acceptance. The download fixture performs a real native download with public bytes and an existing-name collision. Manager fixtures exercise actual isolated controllers, document reuse, modal/lock hiding, locked-command rejection, password unlock and actual-origin IPC denial. None creates fake OS message notifications or claims real WhatsApp media/authentication from fixture success. See RELEASE_0.3.0.md.
