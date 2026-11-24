> Current status (0.1.2, 2026-11-24): The Windows browser-profile path representation was fixed. The user confirmed login persistence through three restarts in Edge, native WebView2, minimal Tauri and the main application. Earlier pending/failure entries below are historical. Three distinct phone accounts have not been confirmed. Production UI and SQLite remain outside this fix. See [the release investigation](AUTH_PERSISTENCE_0.1.2.md).

# Plan and status

Current gate: the user reported real login loss on restart. The 0.1.1 persistence investigation and lifecycle changes are documented in SESSION_PERSISTENCE.md. Production UI and SQLite remain blocked until user-verified real-session acceptance passes. Automated storage, clean browser-exit regressions, native checks and the rebuilt 0.1.1 release packages pass; these do not establish authenticated login continuity.

1. Inspect workspace/toolchain; official Tauri Vue/TypeScript scaffold — done. Empty workspace; Node 26.12.1/npm 11.20.0; Rust/Cargo absent.
2. Three persistent child-WebView profile probe — source implemented. Rust stable is installed permanently, the local x86 .NET native-image fault was repaired, and MSVC/Windows SDK installation completed successfully. Debug compilation and app launch now pass; three native panels load the provider login page. Isolation and linked-session restart tests remain pending.
3. Prove three WhatsApp logins, isolation, restart persistence and remote IPC denial — pending. No supported platform claimed.
4. After compatibility gate passes: Phase 1 SQLite repository, complete account IPC, trusted shell, settings, themes and safe profile deletion — pending, gated by step 3.
5. MVP automated checks and manual release acceptance — pending.

This deliverable is a compatibility prototype, not the completed MVP. Phases 2–4 are out of scope. No credentials, QR codes, messages or cookies are inspected.
