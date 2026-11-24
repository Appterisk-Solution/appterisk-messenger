# Signing and key recovery

## Tauri updater signatures

The repository owner authorized creating one production key using `scripts/signing-key.ps1 -Action Init -ProjectRoot <repository>`. Init refuses to overwrite an existing key. The encrypted private key is under the owner's `.tauri/appterisk-messenger/production.key`; its random encryption password is stored separately as current-user DPAPI ciphertext. The directory ACL grants the owner access and disables inherited access. No private value is printed. `src-tauri/update-public-key.txt` and the updater configuration contain only the public key.

Build locally from the repository with `scripts/signing-key.ps1 -Action Build -ProjectRoot <repository>`. It decrypts the password only for the child build environment and removes signing environment variables afterward. Tauri writes version-bound `.sig` files after final installer packaging. `release-tools` verifies public-key signatures and the signed version. Public test fixtures use a separate disposable key; its private material is outside the repository and is never embedded in a distributed build.

The current DPAPI password store is not a portable offline recovery backup. The owner must transfer the encrypted private key and its recovery password to independently encrypted offline storage, disconnect it and verify recovery before setting `OFFLINE_KEY_BACKUP_CONFIRMED=true`. Do not paste the password into chat, commit it, or print it into terminal/CI logs. Loss of both the password and the current Windows user profile can make the key unrecoverable. Offline backup is pending owner action, not an automated pass.

GitHub Actions environment `release-signing` needs `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` secrets. Configure required reviewers on `release-signing` and `release-publish`; signing credentials must never be available to fork/PR CI. Do not put secrets in repository variables or workflow YAML. The initial owner-signed beta bootstrap can distribute already-signed local artifacts without putting the private key on GitHub.

Key rotation requires a transition build signed with the old key that embeds the new public key, followed by releases signed with the new key. Keep the old key offline until the transition is complete. A compromised key requires stopping feeds and an independently trusted recovery distribution; automatic key recovery/rollback is not implemented.

## Windows Authenticode

No commercial publisher certificate is currently configured. Tauri updater signing is operational independently; Windows EXE/NSIS/MSI publisher signatures must be reported unsigned until actual verification succeeds. Never substitute a self-signed certificate for a trusted publisher.

The protected release workflow supports an owner-provided exportable certificate via `WINDOWS_CERTIFICATE_BASE64` / `WINDOWS_CERTIFICATE_PASSWORD`, imported only in the ephemeral Windows runner. The Tauri bundler signs application and installers using SHA-256 and the configured trusted timestamp server before generating updater signatures. `verify-authenticode.ps1` requires a valid Windows signature chain and timestamp certificate. Hardware/remote signing providers require the appropriate provider integration before use; none is claimed deployed. Publisher identity comes from the issuer-verified certificate.

Trusted stable publishing fails if certificate configuration or offline backup confirmation is missing. Protect release environments and review workflow changes. An Authenticode-unsigned experimental prerelease must be labeled clearly and must not advance Stable.
