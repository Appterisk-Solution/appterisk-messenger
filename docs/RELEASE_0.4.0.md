# v0.4.0 verification - incremental Telegram stage

Date: 2026-11-24, Asia/Kuala_Lumpur. Branch: feature/multi-platform. Stable checkpoint: v0.3.0-stable.

This is incremental Phase A/B/C validation, not completion of the requested final multi-platform release. Telegram and Discord remain experimental. Subsequent providers, custom websites, grouping/search/duplication, extended navigation and suspension/resource work remain scheduled after shared-architecture acceptance.

## Automated results

TypeScript, ESLint and 17 frontend tests passed. Rust formatting, all-target Clippy with warnings denied and 21 Rust tests passed. Synthetic isolation passed three profiles through three cold restarts (run 41b16c00-b2c0-4af6-a5d9-533afe2f868d). The four-profile manager passed seed, three cold restarts and removal (run 44c2ec59-4fab-4752-88e2-52918ab34873). Native public download and collision preservation passed (run e15e0fb5-ad65-4caf-8c6c-cd7514ef3d08). Unit checks cover exact platform origins, blocked credential-bearing URLs and unavailable platforms, schema-v2 migration preservation, permission isolation, foreign keys and UUID/profile uniqueness. Existing regression suites are retained. Native manager now covers three WhatsApp profiles and one separate unlinked Telegram profile across switches and restarts. Public test fixtures never establish real authentication compatibility.

## Manual results

- v0.3.0 baseline: passed by user confirmation, including three accounts through three full restarts.
- v0.4.0 WhatsApp upgrade authentication and three restart cycles: passed by user confirmation.
- Telegram real authentication and three full restarts: passed by user confirmation. Multiple Telegram identities, incoming/background delivery, notifications/clicks, activity clearing and media remain pending.
- Native UI visual review: English platform grid and experimental warning inspected in a visible isolated empty-shell window, then closed normally. An initial hidden-window attempt was abandoned. Review caught hidden customization controls; corrected to a rendered container and verified through frontend tests. Final native visual check confirmed the color and notification controls render correctly; the dialog scrolls when content exceeds the window height.
- MSI upgrade, installer signing, automatic updates, macOS/Linux: not verified/implemented.

Schema 3 cannot be opened by v0.3.0. Browser authentication databases are not inspected, copied or migrated. Installation preservation checks use file count/size/mtime only. Existing permissions and account notification preferences remain independent and unchanged.

The existing Chromium Chrome_WidgetWin_0 shutdown warning 1412 is still emitted by fixture processes. Storage tests and normal BrowserProcessExited acknowledgments passed; no force termination was used.

The unchanged independent native/minimal-Tauri controls also passed formatting, Clippy and test harnesses; they contain zero unit cases. The production shell now uses provider-neutral help and localized platform status/permission copy in English, Simplified Chinese and Traditional Chinese. Native provider marks use application-owned letter badges rather than copied brand artwork.

## Build and installer upgrade

Final production build generated the x64 executable, NSIS and MSI packages. NSIS upgrade from installed 0.3.0 to 0.4.0 exited 0; all 3,303 existing profile files retained count, size and mtime during installation. The installed executable matches the final package with the documented Tauri NSIS bundle marker. Browser authentication databases were not read or copied. Artifact hashes and installer evidence are in release-evidence-0.4.0. Real post-upgrade authentication remains pending user confirmation.

On first installed launch, schema 2 migrated to 3. Metadata-only SHA-256 comparisons passed for all three accounts (including names, original UUIDs/profile keys, platform, icons, order, notification flags and timestamps), desktop settings/host lock configuration and origin permissions. Last-active selection is allowed to change normally on startup; it is excluded from settings comparison. Native logs confirm all three original profile paths match and incognito remains off. These checks do not read browser storage or establish that real provider sessions remain authenticated.

## Telegram checkpoint confirmation

The user replied "test pass" to the explicit Telegram and all three existing WhatsApp accounts through three complete restarts checkpoint. Record authentication persistence and post-upgrade WhatsApp sessions as user-confirmed. Telegram notifications, media, unread clearing and multiple Telegram identities remain independently pending; this response does not establish those results. Proceed to the next incremental Discord stage, keeping Telegram experimental.

## Phase C: Discord validation stage

After user-confirmed Telegram and WhatsApp restart acceptance, Discord was enabled experimentally using the same isolated native manager. Messenger, Instagram, Slack, Teams, Gmail and custom websites remain unavailable pending their sequential implementation and authentication review. No new SQLite migration is required; existing profile identifiers and metadata remain unchanged.

Official entry, supported-browser and normal authentication requirements were reviewed in primary Discord documentation (linked in PLATFORM_COMPATIBILITY.md). Strict origin policy allows only https://discord.com navigation/permission requests; no third-party OAuth or native-protocol bypass was added. Voice, media, notifications and real Discord authentication remain unverified. Frontend checks passed with 18 tests; Rust formatting/Clippy and 22 unit tests passed. Native and installer checks for this stage are recorded separately under release-evidence-0.4.0/discord to retain the original Telegram evidence.

Discord-stage native results: five-profile manager passed seed, three cold restarts and removal (3522b9b8-39f1-4d25-b8a7-c2b94ee8fda7); storage isolation passed three cold restarts across three disposable profiles (efc90021-7233-40a6-90ea-bc55ebf96e64); native public attachment download/no-clobber fixture passed (82029843-c060-4b1e-829f-e0cef5a722cb). None of these are real Discord login/message tests. The original Telegram binary/installers are retained under src-tauri/target/release/staged-telegram-326a956 and their archived paths and original hashes are recorded in telegram-artifact-archive.json.

Discord-stage production packaging generated executable, NSIS and MSI artifacts. The NSIS 0.4.0 refresh exited 0 and preserved all 4,400 existing profile files by count/size/mtime. The installed payload hash matches the package. Metadata SHA-256 comparison passed for all four existing accounts, desktop settings and origin permissions; schema remained 3. The installed app was reopened for user acceptance. Discord login and post-refresh real sessions remain pending user confirmation. Original Telegram-stage evidence is unchanged.
