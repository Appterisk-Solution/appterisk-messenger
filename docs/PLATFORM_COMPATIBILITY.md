# Platform compatibility - staged v0.4.0

| Service | Availability | Evidence and limitations |
|---|---|---|
| WhatsApp | Verified v0.3.0 baseline | User confirms all desktop acceptance plus three accounts through three full restarts. v0.4.0 upgrade/manual regression pending. Existing profile paths unchanged. |
| Telegram | Experimental | Official WebK https://web.telegram.org/k/ (linked by https://telegram.org/apps). Normal QR or phone login is performed by the user on the official site. No real authentication/persistence, notifications, unread clearing or media acceptance claimed yet. Generic activity dot only from genuine native notifications. |
| Discord | Unavailable | Next after Telegram validation; URL/authentication review and implementation pending. |
| Messenger / Instagram | Unavailable | Proposed integration; Meta embedded authentication not reviewed or bypassed. |
| Slack / Teams / Gmail | Unavailable | Workspace/SSO/embedded authentication restrictions require independent review. |
| Custom websites | Unavailable | Custom URL and permission policies not implemented yet. |

Only WhatsApp and experimental Telegram accounts can currently be created. Unavailable entries are disabled. Each enabled account has an independent UUID and persistent native profile. No test copies real authentication data. Public-origin loading and remote IPC rejection do not establish real provider compatibility.

## Telegram manual acceptance

Add Telegram from the account dialog and authenticate normally. Test two distinct Telegram accounts if available. Quit fully and reopen three times, confirming separate sessions. Switch between WhatsApp and Telegram without reload; check incoming and background messages, native notification routing/clicks, generic unread behavior and supported attachments. Record provider restrictions or failed steps explicitly. Do not enable subsequent providers until this shared architecture is validated.
