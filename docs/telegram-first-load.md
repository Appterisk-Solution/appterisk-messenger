# Telegram first-load compatibility fix — 0.5.1

On Windows/WebView2, a fresh Telegram Web K profile reproducibly loaded a blank
document. Native navigation and document readiness completed; the login interface
did not render. Refreshing was the user's workaround. Empty-profile diagnostics
also reproduced the failure. Downloaded script responses were successful and the
temporary debug instrumentation saw no page-level errors. Delaying navigation
until after positioning/showing the controller did not resolve it and was removed.

The same native manager with Telegram's official Web A client rendered controls on
first navigation. Version 0.5.1 therefore uses `https://web.telegram.org/a/` for new
Telegram profiles. This is a targeted compatibility workaround; the internal cause
of Web K's stalled bootstrap has not been proven. It is not an authentication bypass.

The selected client is pinned in an application-owned text file in the profile
root. Existing browser directories without this marker retain Web K, so no existing
session is migrated to a different client. The UUID, origin, profile directory,
browser storage and permission/IPC policies are unchanged. The app never reads
browser databases or authentication values to make this choice. It does not inject
production scripts or automatically reload Telegram.

Regression checks:

- Unit check: fresh profiles choose A; repeated opens retain A after WebView2 creates
  its directories; legacy profiles retain K; invalid marker values fail closed.
- `node scripts/telegram-first-load-tests.mjs`: three disposable Telegram profiles,
  each opened twice in separate processes. Requires visible page controls without
  reload and stable account identity; repeated selection preserves the document.
- Existing native manager checks cover independent profiles, restart metadata,
  reuse, visibility, resize, dialogs, removal, lock and remote IPC denial.

These tests do not link a Telegram identity. Real Web A authentication, media and
notification acceptance remains a manual test. Existing real user profiles are not
used by diagnostics. No change is uploaded into the 0.5.0 Store submission already
under certification; 0.5.1 requires its own subsequent release/submission.
