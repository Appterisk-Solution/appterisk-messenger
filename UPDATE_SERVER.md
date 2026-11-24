# GitHub update manifests

Repository: https://github.com/Appterisk-Solution/appterisk-messenger

Channel manifests live on the separate `update-channels` branch:

- Stable: https://raw.githubusercontent.com/Appterisk-Solution/appterisk-messenger/update-channels/stable/update.json
- Beta: https://raw.githubusercontent.com/Appterisk-Solution/appterisk-messenger/update-channels/beta/update.json

The stable release also maintains a root `update.json` alias on that branch. Beta never overwrites the stable manifest. These exact endpoints are compiled into Rust; optional `APPTERISK_STABLE_ENDPOINT` / `APPTERISK_BETA_ENDPOINT` build variables accept HTTPS endpoints only. No GitHub token is included in the app. The proposed custom domain is not deployed or assumed to exist.

Artifacts live in GitHub Releases at version-specific URLs. The generator reads actual NSIS installer bytes and its actual Tauri signature, cryptographically verifies them, binds the signature to the announced version, validates the version/channel/URL and writes an RFC3339 timestamp. The custom target is `windows-x86_64-nsis`; no MSI fallback is supplied. SHA256SUMS and artifact metadata accompany the release.

Run `node scripts/package-release.mjs 0.5.0 beta` after a successful signed build. Never edit signatures, versions or artifact filenames manually. `publish-release.mjs draft` uploads to a draft without changing feeds. `publish` makes the approved release visible, downloads the public NSIS asset and verifies its checksum against the signed local bytes, then commits the channel manifest last. If any step fails, the feed is left unchanged. No placeholder manifest is published.

Until a channel has its first approved release, its URL may return 404. This is an update-check error and must not interrupt messaging. This initial beta is not a trusted stable Windows publisher release. Do not use GitHub's latest-release endpoint to choose channels.

The verified packaging step copies installer/signature pairs to space-free asset names (spaces become hyphens) without changing signed bytes. All manifest references, checksums and artifact records are generated from those copied names before upload. This avoids GitHub asset-name normalization ambiguity.

The experimental v0.5.0 beta is published at the Beta endpoint above. Stable is intentionally unpublished pending release acceptance and signing requirements. The `updater-e2e-fixtures` branch and `0.5.1-e2e.1` prerelease are separate disposable-key synthetic tests; never configure the production application to trust that test key or feed.
