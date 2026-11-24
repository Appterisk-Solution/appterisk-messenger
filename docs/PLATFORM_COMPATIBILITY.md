# Platform compatibility - staged v0.4.0

| Service | Availability | Evidence and limitations |
|---|---|---|
| WhatsApp | Verified v0.3.0 baseline | User confirms all desktop acceptance plus three accounts through three full restarts. v0.4.0 upgrade and three-restart authentication passed by user confirmation; new-stage regressions remain separate. Existing profile paths unchanged. |
| Telegram | Experimental | Official WebK https://web.telegram.org/k/ (linked by https://telegram.org/apps). Normal QR or phone login is performed by the user on the official site. Real login persistence across three restarts passed by user confirmation. Multiple Telegram identities, notifications, unread clearing and media acceptance remain pending. Generic activity dot only from genuine native notifications. |
| Discord | Experimental | Official https://discord.com/app, restricted to https://discord.com. Normal email/verified phone plus password or mobile QR login. No embedded-login, restart, notification, media or voice compatibility claimed before real acceptance. Native notification activity dot only. |
| Messenger / Instagram | Unavailable | Proposed integration; Meta embedded authentication not reviewed or bypassed. |
| Slack / Teams / Gmail | Unavailable | Workspace/SSO/embedded authentication restrictions require independent review. |
| Custom websites | Unavailable | Custom URL and permission policies not implemented yet. |

WhatsApp, experimental Telegram and experimental Discord accounts can currently be created. Unavailable entries are disabled. Each enabled account has an independent UUID and persistent native profile. No test copies real authentication data. Public-origin loading and remote IPC rejection do not establish real provider compatibility.

## Telegram manual acceptance

Add Telegram from the account dialog and authenticate normally. Test two distinct Telegram accounts if available. Quit fully and reopen three times, confirming separate sessions. Switch between WhatsApp and Telegram without reload; check incoming and background messages, native notification routing/clicks, generic unread behavior and supported attachments. Record provider restrictions or failed steps explicitly. Do not enable subsequent providers until this shared architecture is validated.

## Discord review and acceptance

Discord documents [normal desktop/browser login](https://support.discord.com/hc/en-us/articles/360057027354-How-to-Log-In-to-your-Discord-Account), including email or verified phone plus password and mobile QR login. Its [browser requirements](https://support.discord.com/hc/en-us/articles/213491697-What-are-the-OS-system-requirements-for-Discord) list supported desktop browsers; these do not independently certify embedded WebView2 compatibility. The official https://discord.com/app entry was checked. Same-origin /login and /channels paths are allowed; third-party identity origins and discord:// are not granted embedded navigation. Users perform all authentication, CAPTCHA and verification themselves. No identity spoofing or credential transfer is provided.

Discord separately documents [voice encryption/client requirements](https://support.discord.com/hc/en-us/articles/38025123604631-Minimum-Client-Version-Requirements-for-Voice-Chat). Runtime version alone does not establish calling compatibility; actual voice/video acceptance is pending. Attachments opening external CDN links may use the system browser under the existing safe external-link policy; native download behavior needs manual verification. No private Discord API or message scraping is used.

Add a Discord account and sign in normally. Quit fully and reopen three times. Check the pre-existing WhatsApp and Telegram accounts too. Independently verify multiple Discord identities, incoming/background messages, native notifications/click routing, activity indicators and attachments before changing Discord from experimental. No numeric Discord unread grammar has been validated, so title text is ignored for counting.
