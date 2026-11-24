# Account manager architecture — 0.2.0

The bundled Vue/TypeScript shell uses Pinia for UI state and Tailwind plus application CSS for presentation. It never renders chats or reads provider storage. Rust owns SQLite, permanent profile keys and native WebViews. `database.rs` holds the metadata repository and transactional migration; `accounts.rs` holds the serialized manager; `windows_profiles.rs` isolates native WebView2 integration. Future non-Windows implementations require their own validated adapter; no compatibility is claimed.

## Metadata

`accounts` stores id, name, platform, profile_key, icon, display_order, notifications_enabled, created_at, updated_at and last_active_at. `application_settings` stores the validated preference record. `schema_migrations` records the applied schema version and timestamp, paired with SQLite user_version. A newer schema is rejected rather than reset. Transactions protect migration, ordering, removal/selection cleanup and activation metadata. SQLite runs locally with foreign keys, WAL and a bounded busy timeout.

New accounts get UUID v4 once. A profile key is `account:<UUID>` or `compatibility:<original UUID>`; keys are validated against the account's permanent id. Browser paths are derived only in Rust. Adoption requires an existing known compatibility directory and an explicit shell action. It inserts metadata referencing that physical directory without opening/copying authentication databases. Removal closes only that account, removes its metadata and leaves the profile directory intact.

## Native manager

One async mutex serializes commands, database operations, selection and native lifecycle changes. A process-held OS file lock excludes another app instance; per-profile locks remain held until process exit, including removed profiles. The optional diagnostic mode and production mode are mutually exclusive under the same instance lock. Separate synthetic runs use isolated metadata and profile roots.

The manager lazily initializes a child WebView when an account is first selected. Canonical namespace/UUID containment is checked, then `profile_paths::browser_directory` converts the API-facing representation with `dunce::simplified`. The effective native user-data directory and non-private mode are checked after creation. No identity override, application page script or browser-storage cleanup is installed in production account views.

New children start offscreen and are hidden before layout. Switching hides all initialized children before showing one; existing controllers/documents are reused. DOM ResizeObserver supplies logical content bounds. Dialog opening awaits Rust hiding before Vue draws the overlay. A single layout queue prevents stale UI resize requests overtaking dialog visibility changes. Native visibility, resize and document reuse are checked in the manager regression.

X/Exit uses the retained 0.1.1 coordinated shutdown path. Account and probe controllers close while their WebView2 environments remain retained on the UI thread. The shell stays alive until BrowserProcessExited acknowledgments arrive; timeout leaves it open and reports retryable failure, never kills a browser. Mode switching starts the new process only after account-browser exit; its lock acquisition waits briefly for the previous process to release the app lock. The shell's own default browser closes with final application exit.

## Security boundary

Tauri build-time command manifests generate explicit allow permissions. Only the `main` local WebView gets those and event listen/unlisten permissions. Remote children receive no capabilities, remote entries, creation permissions or event emission privileges. Every application handler also verifies `main` plus the trusted local origin. The actual official remote origin is independently tested for rejection of snapshot, mutation, restart and probe commands.

Provider main-frame navigation requires exact `https://web.whatsapp.com` on port 443 without credentials. Valid HTTP(S) popups use the system browser; other schemes/embedded credentials are rejected. New sensitive permission requests and downloads are denied. Native navigation failures are reported as generic errors without URL/message contents. Account shortcut events originate in Rust and do not grant remote script capabilities.

## Diagnostics and verification

The original probe is an opt-in lazy frontend and retains its metadata-only storage diagnostics. Independent native/minimal controls remain unchanged as baseline comparators. The production bundle excludes debug fixture harnesses. Diagnostics never extract tokens, keys, messages or raw console content.

Unit tests cover migration idempotence, future schema rejection, CRUD/order validation, stable UUID/profile keys, preference persistence/removal cleanup, adoption, path normalization, locking and navigation policy. Frontend tests exercise failed switching, sidebar/order behavior, confirmation and awaited native dialog hiding. Native manager tests exercise three real controllers on disposable profiles through three restarts, switches without reload, actual-origin IPC denial, dialog hiding, resizing and removal isolation. The runner verifies normal browser exit records after process exit. Synthetic storage persistence remains a separate mandatory regression.
