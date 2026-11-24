# v0.4.0 verification - incremental Telegram stage

Date: 2026-11-24, Asia/Kuala_Lumpur. Branch: feature/multi-platform. Stable checkpoint: v0.3.0-stable.

This is Phase A/B validation, not completion of the requested final multi-platform release. Telegram remains experimental. Subsequent providers, custom websites, grouping/search/duplication, extended navigation and suspension/resource work remain scheduled after shared-architecture acceptance.

## Automated results

TypeScript, ESLint and 17 frontend tests passed. Rust formatting, all-target Clippy with warnings denied and 21 Rust tests passed. Synthetic isolation passed three profiles through three cold restarts (run 41b16c00-b2c0-4af6-a5d9-533afe2f868d). The four-profile manager passed seed, three cold restarts and removal (run 44c2ec59-4fab-4752-88e2-52918ab34873). Native public download and collision preservation passed (run e15e0fb5-ad65-4caf-8c6c-cd7514ef3d08). Unit checks cover exact platform origins, blocked credential-bearing URLs and unavailable platforms, schema-v2 migration preservation, permission isolation, foreign keys and UUID/profile uniqueness. Existing regression suites are retained. Native manager now covers three WhatsApp profiles and one separate unlinked Telegram profile across switches and restarts. Public test fixtures never establish real authentication compatibility.

## Manual results

- v0.3.0 baseline: passed by user confirmation, including three accounts through three full restarts.
- v0.4.0 WhatsApp upgrade authentication and three restart cycles: pending.
- Telegram real authentication, three restarts, multiple accounts, incoming/background delivery, notifications and clicks, activity indicators, uploads/downloads: pending.
- Native UI visual review: English platform grid and experimental warning inspected in a visible isolated empty-shell window, then closed normally. An initial hidden-window attempt was abandoned. Review caught hidden customization controls; corrected to a rendered container and verified through frontend tests. Final native visual check confirmed the color and notification controls render correctly; the dialog scrolls when content exceeds the window height.
- MSI upgrade, installer signing, automatic updates, macOS/Linux: not verified/implemented.

Schema 3 cannot be opened by v0.3.0. Browser authentication databases are not inspected, copied or migrated. Installation preservation checks use file count/size/mtime only. Existing permissions and account notification preferences remain independent and unchanged.

The existing Chromium Chrome_WidgetWin_0 shutdown warning 1412 is still emitted by fixture processes. Storage tests and normal BrowserProcessExited acknowledgments passed; no force termination was used.

The unchanged independent native/minimal-Tauri controls also passed formatting, Clippy and test harnesses; they contain zero unit cases. The production shell now uses provider-neutral help and localized platform status/permission copy in English, Simplified Chinese and Traditional Chinese. Native provider marks use application-owned letter badges rather than copied brand artwork.

## Build and installer upgrade

Final production build generated the x64 executable, NSIS and MSI packages. NSIS upgrade from installed 0.3.0 to 0.4.0 exited 0; all 3,303 existing profile files retained count, size and mtime during installation. The installed executable matches the final package with the documented Tauri NSIS bundle marker. Browser authentication databases were not read or copied. Artifact hashes and installer evidence are in release-evidence-0.4.0. Real post-upgrade authentication remains pending user confirmation.

On first installed launch, schema 2 migrated to 3. Metadata-only SHA-256 comparisons passed for all three accounts (including names, original UUIDs/profile keys, platform, icons, order, notification flags and timestamps), desktop settings/host lock configuration and origin permissions. Last-active selection is allowed to change normally on startup; it is excluded from settings comparison. Native logs confirm all three original profile paths match and incognito remains off. These checks do not read browser storage or establish that real provider sessions remain authenticated.
