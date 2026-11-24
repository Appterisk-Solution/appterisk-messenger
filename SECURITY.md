# Security

Remote messaging pages cannot invoke updater commands. Only the local `main` shell is authorized; installation additionally requires an unlocked application and explicit confirmation. There is no arbitrary endpoint, executable path or signing-key IPC.

Updates use official tauri-plugin-updater 2.14.1, HTTPS manifests, a production public key, signed-version enforcement, semantic version/channel checks and a fixed GitHub repository artifact allowlist. Only Windows x64 NSIS installations can install automatically. MSI packages remain manual installs; they are never silently converted to NSIS.

Release notes are text, not HTML. Downloads are cancellable and limited to 256 MiB; downloads/checks have network timeouts. Only one update operation runs at a time. Stable rejects prereleases and neither channel permits downgrades.

Install preparation blocks queued account mutations, rejects active downloads/permission prompts, gracefully closes controllers and waits for normal browser-process acknowledgments. SQLite is checkpointed/closed and profile/instance locks released before the official installer is invoked. Shutdown failure aborts installation. Installer-start failure attempts to reopen metadata and lets the user select an account to resume. Profiles are never copied, reset or deleted.

The encrypted updater private key and DPAPI-protected password live outside the repository. Only public keys and public disposable signature fixtures are committed. Production keys are never uploaded as build artifacts. Never post browser authentication files, private signing keys, passwords or message content in public issues. Use private vulnerability reporting if enabled for the repository.

Tauri artifact signatures do not supply Windows publisher identity. These initial experimental packages are Authenticode-unsigned. Trusted stable publishing is blocked until genuine certificate signing, timestamp verification and owner-confirmed offline recovery backup are configured. Full installed-updater end-to-end and real-account acceptance remain pending; see RELEASE_VERIFICATION_0.5.0.md.
