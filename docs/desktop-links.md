# Windows messaging links — 0.5.5

Appterisk accepts common `whatsapp://send?phone=...&text=...` and
`tg://resolve?domain=...&post=...` links. It also accepts equivalent `https://wa.me/`,
`https://api.whatsapp.com/send`, `https://t.me/` and `https://telegram.me/` chat links
through Settings → General → Open link, and from supported links inside account views.

When a link arrives, the trusted shell asks which account to use. Navigation stays
on the official provider's web origin and retains the account's existing browser
profile. Telegram uses that profile's pinned A/K client. WhatsApp text is a draft;
Appterisk never submits messages. A link cannot unlock the app. Closing the chooser
discards the pending request. Destinations and draft text remain in process memory,
are not logged, and are not saved to the application database. Unknown parameters,
duplicate parameters, authentication/proxy/payment actions and arbitrary URLs are
rejected. This is intentionally not a full implementation of every provider URI.

The standalone Windows build registers Appterisk-specific ProgIDs and a
RegisteredApplications capability entry for `tg` and `whatsapp` after launch. It
does not overwrite another app's scheme registration or Windows UserChoice. Open
Settings → General → Windows Default apps and select Appterisk Messenger for the
desired protocols. The MSIX build declares the two protocols in its manifest and
leaves registration to Windows. Existing notification activation remains separate.

Plain HTTPS links clicked in an external browser may stay in that browser or first
display its Open app prompt. Appterisk cannot claim verified website associations
for Telegram's or WhatsApp's domains. There is no browser extension in this release.

Validation includes parser tests, a Vue account-choice test, and a native disposable
profile check for cold command-line receipt, the existing second-instance callback,
account mismatch, stale requests, app lock and remote IPC denial. Run
`npm run test:links`. The native check does not change Windows defaults or link a
real messaging identity. Installed Windows association/browser handoff, actual
chat resolution after login and Store activation still require manual verification.
Windows registration and native checks do not establish macOS URI activation support.

References:

- https://v2.tauri.app/plugin/deep-linking/
- https://learn.microsoft.com/en-us/windows/apps/develop/windows-integration/default-apps-platform
- https://learn.microsoft.com/en-us/windows/apps/develop/launch/web-to-app-linking
- https://core.telegram.org/api/links
- https://faq.whatsapp.com/5913398998672934
