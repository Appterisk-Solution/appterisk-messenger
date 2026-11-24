# Release verification — 0.2.0

Date: 2026-11-24 (Asia/Kuala_Lumpur). Platform: Windows x64, WebView2 Evergreen. Before implementation the user confirmed three distinct 0.1.2 accounts survived three full restarts without an issue. Existing profiles were not deleted, copied or migrated.

After installation, the user separately confirmed the three adopted accounts stayed logged in through three full restarts of Appterisk 0.2.0, without another QR scan. The exact response was “All three stayed logged in through three restarts”. This passes the requested real authentication acceptance on the tested Windows machine.

## Implemented behavior

The default application is the real Rust/SQLite/native-WebView account manager described in README and ARCHITECTURE. The 0.1.2 path normalization and browser-exit acknowledgments are retained. Existing compatibility accounts require explicit in-place adoption. No privileged command capability is granted to remote views. One controller per initialized account is reused during switching; inactive controllers remain alive and hidden.

## Verified checks

- Eight frontend tests, TypeScript checking and ESLint passed.
- Twelve main Rust unit tests passed, including SQLite migration/reopen, account CRUD/order/UUID, adoption, settings, navigation, locks and Windows paths.
- Main Rust formatting and Clippy with all targets and warnings denied passed. Independent control-workspace formatting/Clippy/tests also passed (that workspace currently contains no unit test cases).
- Mandatory synthetic isolation passed seed plus three cold restarts across three profiles, with durable cookies/localStorage/IndexedDB, Blob integrity, synthetic non-extractable key usability, expected session-cookie expiration, IPC denial and native shutdown. Run `372e6c3f-638c-47f5-b34c-9c8c6ee5fd1d`.
- Native manager integration passed seed, three restart cycles and removal. It verified stable SQLite UUID/order/last-selection data, in-place synthetic legacy adoption, three isolated native controllers, exactly one visible child, document-marker survival during switching, dialog hiding, native logical-pixel resizing, sibling retention on removal, actual remote-origin command denial and normal BrowserProcessExited acknowledgments. Final run: `ca4872ae-e67b-4b60-a8b8-0e6a784551cd`. Results are captured in `release-evidence`.
- Actual `https://web.whatsapp.com` views rejected `account_snapshot`, `account_add`, `restart_mode` and `reload_probe`. These checks were independent of local fixture IPC rejection and used disposable unlinked profiles.
- Desktop empty state and dark/light settings were visually inspected through the computer-use skill. The inspected shell listed three existing profiles available to adopt; none was automatically adopted or opened by the visual check. System theme was restored afterward.

The Chrome_WidgetWin_0 warning 1412 still appears after normal native exits. Native exit records are checked independently; the warning is not treated as proof of storage failure or successful authentication.

## Build and upgrade

The production build passed. Artifacts are the Windows x64 executable and unsigned NSIS/MSI installers under `src-tauri/target/release`. The NSIS 0.1.2 → 0.2.0 upgrade completed with exit code 0. All 2,228 pre-existing profile files retained their counts, lengths and modification timestamps. See `release-evidence/nsis-upgrade.json`.

After the final focus-transfer adjustment, the rebuilt 0.2.0 package was reapplied and again preserved all 2,228 profile files. Its installed executable matched the expected final NSIS payload hash: the only expected difference from the raw executable is Tauri's three-byte `UNK` → `NSS` bundle-type marker. That patch is applied in memory for verification; no executable is rewritten by the test. See `release-evidence/nsis-final-package.json` and `release-evidence/artifacts.json`.

`npm run test:upgrade` runs the local verification script to install the 0.1.2 baseline and apply the 0.2.0 update. It refuses to downgrade an existing newer installation. `npm run test:upgrade -- --refresh` explicitly verifies a same-version final package without overwriting the original upgrade evidence. Neither app runs during replacement. No browser databases are opened. The upgraded application is left installed at `%LOCALAPPDATA%\Appterisk Messenger\appterisk-messenger.exe` and was opened for user acceptance.

The installer checks verify filesystem preservation; the later independent user confirmation establishes actual three-account authentication continuity on the installed 0.2.0 version. MSI installation/upgrade is not claimed tested. No signing or installer auto-update infrastructure is configured.

## Manual acceptance on 0.2.0

1. Adopt each of the three existing profiles through Add account; verify the intended account identity yourself. **Passed, user confirmed.**
2. Switch repeatedly, rename and reorder; ensure only the selected account is visible and dialogs are unobscured.
3. Close with X and reopen the same installed 0.2.0 executable three times; verify all three logins survive. **Passed, user confirmed.** Last-selection persistence also has automated coverage; the question did not separately ask for a manual last-selection result.
4. Remove a disposable account with confirmation; verify siblings remain logged in. Do not remove a valuable linked account merely to test.
5. Check resize, monitor/DPI changes and reload behavior in the real environment.

The user confirmation covers three-account authentication through three restarts. Other manual steps above remain unconfirmed. Notification delivery, background messages, unread counts, media/calls/downloads, macOS/Linux and MSI upgrade results are not inferred from automated tests. Notification/language preferences are saved metadata; native notification delivery and full shell localization are deferred. Dev-only `@vue/test-utils` transitive dependencies still report moderate npm audit advisories after the compatible audit fix; these are not shipped as application runtime code.
