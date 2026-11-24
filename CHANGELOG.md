# Changelog

## 0.2.0 — 2026-11-24

- Replaced the default compatibility dashboard with a sidebar account manager, empty/loading/error states, account context actions and settings dialogs.
- Added Rust-side SQLite migrations and metadata-only account CRUD, ordering, permanent UUIDs, startup selection, icons and preferences.
- Added a centralized lazy native WebView manager: one isolated controller per initialized account, reuse without switch reloads, inactive hiding, logical-pixel resize and modal blocking.
- Added explicit in-place adoption of original compatibility profiles. Removal retains browser data and does not unlink WhatsApp.
- Preserved 0.1.2 ordinary Windows browser paths, canonical confinement, non-private profiles, native effective-path verification and graceful exit acknowledgment.
- Added process/profile locks, explicit application-command capabilities, actual remote-origin authorization tests, native focused-view accelerators and navigation-failure reporting.
- Preserved the original probe as an opt-in diagnostic mode with orderly restarts between modes.
- Added frontend, database and native manager regression coverage. Release evidence separates synthetic checks from manual authentication and installer verification.

Notifications, unread counts, background delivery and media support are not claimed verified. Language preferences are persisted; full shell localization is not included.

## 0.1.2

Fixed authentication persistence on the tested Windows runtime by converting canonical verbatim browser paths to ordinary absolute paths. Added native and minimal Tauri controls. User-confirmed three-restart tests passed in Edge, both controls and Appterisk; subsequent user confirmation covered three distinct accounts before 0.2.0 implementation.

## 0.1.1

Added metadata-only diagnostics, native persistent-profile verification and coordinated graceful shutdown. Real WhatsApp authentication still failed on this version.

## 0.1.0

Initial three-profile compatibility probe and synthetic isolation fixture.
