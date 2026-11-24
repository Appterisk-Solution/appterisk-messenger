# Privacy

Appterisk Messenger displays official messaging websites through Microsoft WebView2. Selecting or restoring an account connects to that provider, which processes information according to its own policies: [WhatsApp](https://www.whatsapp.com/legal/privacy-policy), [Telegram](https://telegram.org/privacy), and [Discord](https://discord.com/privacy).

Account names, IDs, ordering, preferences and permission decisions are stored locally in SQLite. Browser cookies, website storage and linked-device sessions remain in separate local WebView2 profiles. The application metadata database does not store message content, passwords or provider authentication tokens. Removing an account retains its browser profile.

Automatic update checks are enabled by default and connect to GitHub at startup and periodically. Users can disable automatic checks in Settings → Updates. Manual checks and downloads also contact GitHub; GitHub and network infrastructure receive ordinary connection metadata such as the source IP address. See [GitHub's privacy statement](https://docs.github.com/en/site-policy/privacy-policies/github-general-privacy-statement).

Microsoft WebView2 and Windows may make their own runtime, update and diagnostic connections under Microsoft's policies and system settings. See [WebView2 privacy considerations](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/data-privacy).

Appterisk has no separate application backend. Local diagnostic logs may contain application versions, profile identifiers, local paths and non-sensitive browser lifecycle metadata. Diagnostics must not collect authentication secrets or private message content. Review logs before sharing them publicly. Provider website behavior and third-party runtime processing are outside Appterisk's control.
