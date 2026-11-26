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

From Git Bash or PowerShell, `npm run build:signed` runs the local encrypted-key build without changing shell execution policy. `npm run signing:backup -- X:/AppteriskRecovery` is an owner-run helper: it refuses the current system drive and requires an already fully encrypted, protected BitLocker volume before writing recovery material. It does not encrypt a drive or change Windows security settings. Disconnect/store that volume offline and verify recovery; this helper has not been run because no offline destination was supplied. Other encrypted offline media require equivalent owner-controlled procedures. Never commit the recovery directory.

## Code signing policy

Status: the owner submitted the SignPath Foundation application; it was declined for insufficient public reputation, as confirmed by the supplied response on 2026-11-26. No certificate was granted. Existing installers remain Authenticode-unsigned. The owner authorized the MIT license for project-owned source on 2026-11-25. Third-party licenses remain unchanged.

Proposed signing roles: James ([GitHub @ipipowng01](https://github.com/ipipowng01)) as committer, reviewer and release approver. Maintainers must verify actual access, enable MFA and approve each release before requesting production signing. No claim is made that provider MFA or account configuration has been completed.

After approval and successful signing, the intended attribution is: ?Free code signing provided by SignPath.io, certificate by SignPath Foundation?. This is conditional attribution, not a claim that the service currently signs Appterisk. Reapplication would require broader public recognition and a dependency-license/System Library review. See [prepared application](SIGNING_APPLICATION.txt) and [privacy policy](PRIVACY.md).

Only artifacts built from the reviewed public repository may be submitted. Signing must preserve product/version metadata and cover the application executable before bundling, followed by the finished installers. Generate Tauri updater signatures only after Windows signing. Publish newly signed binaries under a new release version; do not replace immutable v0.5.0 assets. Keep release acceptance and cryptographic verification gates in place.

The Microsoft Store MSIX candidate is documented in [STORE_RELEASE.md](STORE_RELEASE.md). Its unsigned submission package does not bypass Windows trust checks; Store certification and Microsoft signing remain pending.
