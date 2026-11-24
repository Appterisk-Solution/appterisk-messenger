# Release guide

1. Review baseline acceptance and unresolved defects. v0.3.0 is stable; v0.4.0 WhatsApp and Telegram restart persistence are user-confirmed. Discord and broader v0.4.0 features remain experimental. The local checkpoint is `feature/pre-updater-checkpoint`; no v0.4.0 stable tag was fabricated.
2. Keep package.json, package-lock.json, Cargo.toml/Cargo.lock and tauri.conf.json versions consistent. Review CHANGELOG and RELEASE_NOTES. Run all configured checks, native suites and updater security tests.
3. Make the encrypted offline key backup and configure protected GitHub environments. Stable requires genuine Authenticode configuration. No private key enters untrusted CI.
4. Run the protected release workflow manually with the approved version/channel. It runs CI, builds signed artifacts, verifies signatures/checksums and uploads a draft release. `release-publish` must require owner approval.
5. Before approval, run clean installation and real signed-baseline-to-newer-version update acceptance in a disposable installation. This full installed-updater E2E test is currently pending; do not infer it from a build or synthetic storage tests. Test interruption, invalid signatures and shutdown failure. Keep evidence separate from real accounts.
6. Approve only after review. Publish the release first and update the appropriate channel manifest last. Never downgrade schemas. MSI upgrade verification, automatic rollback and macOS/Linux are not claimed.

For an initial Authenticode-unsigned experimental beta, a separate owner-signed bootstrap workflow may upload locally signed/verified artifacts to a prerelease and publish Beta only. This is not the protected trusted stable workflow. The production private key stays outside GitHub and all manual acceptance limitations remain visible.

The initial v0.5.0 installation from v0.4.0 is manual because v0.4.0 has no updater. Future supported NSIS versions can use Settings > Updates. Close-to-tray does not bypass update shutdown. If installation fails after browsers close, select an account to resume; never delete profiles. Check schema compatibility before any metadata recovery. Metadata backups use SQLite's consistent backup API before forward migrations and inherit the private application-data directory permissions.

Local diagnostic evidence can include machine paths and opaque profile IDs and is not automatically published. The public source export excludes local release-evidence directories; local Git checkpoints and evidence remain preserved. GitHub CI artifacts provide separate public automated evidence.
