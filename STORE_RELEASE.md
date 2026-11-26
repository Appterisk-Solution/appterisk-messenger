# Microsoft Store MSIX candidate

Prepared 2026-11-26. This is a candidate for certification, not an approved or Store-signed release.

## Reserved product

- Name: Appterisk Messenger
- Store ID: `9NRZ58RTHSGZ`
- Package name: `AppteriskSolutionMSDNBHD.AppteriskMessenger`
- Publisher: `CN=836DCC0B-274C-4946-AD67-90F4ED2B574D`
- Publisher display name: Appterisk Solution (M) SDN BHD (confirmed by owner)
- Package family: `AppteriskSolutionMSDNBHD.AppteriskMessenger_m5wmqj4hbrmm8`
- Architecture/initial minimum: Windows 11 x64, build 22000 or later
- Metadata version: app 0.5.0, MSIX 0.5.0.0

These public package identifiers were read from Partner Center. They are not API credentials. The exact manifest identity must continue matching Partner Center.

## Build

On Windows x64 with Node/npm, Rust, Microsoft C++ Build Tools and the Windows SDK:

```bash
npm ci
npm run build:store
```

The build uses a separate Cargo output directory, the `store-distribution` feature and `tauri.store.conf.json`. It does not require or expose the Tauri production private key or a publisher certificate. It verifies the executable's distribution marker before packaging. MakeAppx validates the schema and creates:

`store-out/Appterisk-Messenger_0.5.0.0_x64.msix`

The adjacent JSON and SHA256SUMS record the actual package metadata and checksum. Staging uses a fresh, explicit file allowlist: app executable, license, collected third-party notices, icon assets and generated manifest. No accounts, profiles, databases, signing material or local logs are included. Existing output packages are not silently replaced; archive the exact prior file explicitly before another packaging run. Dependency notices include available upstream license files and declarations; a full dependency/compliance review is not implied.

This is an **unsigned Partner Center submission artifact**. It is not a directly installable signed release for friends. Microsoft signing follows successful Store certification. Do not replace the GitHub v0.5.0 release assets with this package. Reserve a higher Store package version for later uploads as required by Partner Center.

## Store-specific behavior

- Store update checks/downloads/installations are controlled by Microsoft Store. The external Tauri updater plugin and periodic feed checks are not initialized in this build. Rust rejects updater mutations even if a caller tries to invoke them directly; the settings UI hides those controls.
- Runtime notification registration uses the current package's Windows identity. The protocol is declared in the MSIX manifest; the build does not rewrite the standalone app's notification/protocol registry registrations.
- The startup toggle is unavailable until packaged StartupTask support is implemented and verified. This build does not write a Run registry entry. An existing standalone startup registration is left unchanged.
- Metadata/profile locations remain the original unredirected LocalAppData location. The manifest excludes only `com.appterisk.messenger` from AppData virtualization; existing account UUIDs, confinement checks, browser-facing path normalization and profile locks remain in force. Both installation types must not use a profile concurrently. The existing instance/profile locks refuse conflicting access.
- Packaged installation, uninstall, activation and real-login preservation still require actual testing. Do not claim preservation based on manifest validation alone. Do not uninstall either installation to troubleshoot login persistence.
- WebView2 Evergreen must be present. The initial target is Windows 11, which normally includes the runtime; removal or damage still needs testing. No separate Chromium engine is shipped.

## Certification capability explanations

`runFullTrust`: Appterisk is a native Win32/Tauri desktop program using native WebViews, local SQLite metadata, system tray, downloads and desktop notifications. It runs as the current user; it does not request administrator elevation.

`unvirtualizedResources`: needed to retain the same on-device metadata and independent browser profiles when an existing standalone user starts the Store edition, and to keep profiles from being silently removed with the package. Only the existing Appterisk LocalAppData directory is excluded. No authentication material is exported, copied or uploaded. This restricted capability requires Microsoft's review; approval is not assumed.

## Submission draft

Category: Social (or the closest available messaging/communication category).

Short description: Manage independent messaging accounts in one Windows desktop application.

Description: Appterisk Messenger brings separate messaging accounts into one desktop window using official provider websites in independent native browser profiles. Switch accounts from the sidebar, customize account names and appearance, and manage local preferences. WhatsApp is supported in the current Windows baseline; Telegram and Discord are experimental and have separately documented limitations. An internet connection and each provider's normal login are required. Appterisk is an independent project and is not affiliated with or endorsed by WhatsApp, Meta, Telegram or Discord. Provider features, availability and terms remain controlled by each provider.

Privacy URL: https://github.com/Appterisk-Solution/appterisk-messenger/blob/main/PRIVACY.md

Support URL: https://github.com/Appterisk-Solution/appterisk-messenger/issues

Notes for certification: Please review the restricted capability explanations above. No Appterisk account is required. Add an account and link it using the provider's official authentication flow. A reviewer must use their own authorized provider account; no user credentials or private profiles are supplied. No purchases, unofficial messaging APIs or application backend are used. Store updates replace the standalone edition's in-app installer flow.

Before submission, complete Partner Center's actual age-rating questionnaire, supported markets, accessibility declarations, listing screenshots and capability questions truthfully. Do not mark unverified features as supported. Screenshots must not contain private chats or user identities. Do not invent a privacy contact address or declare certifications on the owner's behalf.

## Acceptance before public release

1. Validate the package through MakeAppx and inspect the contents and executable imports.
2. Run frontend checks, Rust formatting/Clippy/tests in default and Store configurations, and native synthetic profile regression checks.
3. Test installation through a supported signed private Store/testing route. Do not install a test root certificate, disable Smart App Control or enable Developer Mode automatically to force this unsigned artifact to install.
4. Close the standalone application through tray → Quit before launching the Store app. Verify the Store identity/AUMID, data paths and single-instance protection.
5. Verify actual logins for existing accounts through three complete restarts, account metadata/settings preservation, notifications and notification-click activation, resizing, downloads, external links and tray behavior.
6. Test Store upgrade and uninstall/data retention with disposable profiles first. Verify the NSIS edition remains functional separately.
7. Run the Windows App Certification Kit against the installed candidate and review its full report. MakeAppx validation is not WACK certification.
8. Submit for certification and verify an approved Store install before asking friends to install it. Public publication is a separate owner-reviewed step.

## Local verification on 2026-11-26

Passed: TypeScript, ESLint, 23 frontend tests, Rust formatting, default and Store Clippy with warnings denied, 24 default Rust tests and 25 Store Rust tests. The Store-feature native manager fixture passed five independent profiles through three restarts and removal, view reuse/visibility, lock behavior, normal browser exit, and remote-origin IPC denial (run `4af20161-4248-4bf7-a8aa-ef8dc2dd9d24`). The native desktop fixture passed its public synthetic download/no-clobber checks (`61c81453-7017-45b7-b189-f136fa429a1c`). Default native storage isolation passed three profiles through three restarts (`07f17683-46be-4d3f-bf67-792485645281`). Chromium shutdown warning 1412 appeared, while the fixtures' normal browser-exit checks passed.

The release executable reports the Store distribution feature, and PE import inspection found Windows system DLL dependencies rather than a missing application-shipped DLL. MakeAppx schema validation and explicit package identity/content/hash verification passed for the initial candidate. Final package details are recorded beside the final artifact. These checks did not install the MSIX.

Pending: actual packaged installation/WACK, Store certification, Store notification identity/activation, runtime availability on a clean machine, packaged upgrade/uninstall behavior and owner-confirmed real provider authentication. No Windows security settings or certificate trust stores were changed for testing. The existing installed app and valuable profiles were not used to manufacture evidence.

## Signing status

SignPath Foundation declined the free application because the project lacked sufficient public reputation, according to the owner's supplied response. MIT licensing remains in place. No Windows publisher certificate was issued through that application. Microsoft Store is the alternative being prepared; no Store approval or signed installation is claimed yet.

## References

- https://learn.microsoft.com/en-us/windows/msix/desktop/desktop-to-uwp-manual-conversion
- https://learn.microsoft.com/en-us/windows/msix/desktop/flexible-virtualization
- https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/code-signing-options
- https://learn.microsoft.com/en-us/windows/apps/publish/publish-your-app/app-package-requirements
