# Appterisk Messenger 0.5.0 - experimental updater release

- Adds Settings > Updates with Stable/Beta preferences, manual checking, download progress, cancellation, release notes and explicit Install and Restart confirmation.
- Uses official Tauri 2 updater signatures, signed-version verification, HTTPS-only sources and GitHub-hosted channel manifests. NSIS automatic installs only; MSI remains manual.
- Preserves account UUIDs, existing profile paths, SQLite schema 3 and desktop preferences. Graceful WebView2 shutdown precedes installer handoff.
- Adds a clear Remove account button in Settings > Accounts with confirmation; browser profiles are retained. No account was removed automatically.
- Adds protected release workflows and separate artifact/signing verification.

This is experimental: Windows publisher signatures are not configured, full installed automatic-update E2E/manual acceptance is pending, Discord acceptance remains pending, and offline key recovery backup requires owner action. This prerelease must not advance Stable. No authentication/session-preservation pass for v0.5.0 is fabricated. v0.4.0 WhatsApp and Telegram restart results remain historical user-confirmed evidence.
