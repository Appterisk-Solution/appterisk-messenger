//! Provider policy only. Remote content never receives shell privileges.
use serde::Serialize;
use tauri::Url;

#[derive(Clone, Copy, Serialize)]
pub struct Platform {
    pub id: &'static str,
    pub name: &'static str,
    pub entry_url: &'static str,
    pub origin: &'static str,
    pub symbol: &'static str,
    pub status: &'static str,
    pub available: bool,
    pub limitations: &'static str,
}
pub const REGISTRY: &[Platform] = &[
    Platform {
        id: "whatsapp",
        name: "WhatsApp",
        entry_url: "https://web.whatsapp.com/",
        origin: "https://web.whatsapp.com",
        symbol: "WA",
        status: "verified",
        available: true,
        limitations: "Three-account authentication across the v0.4.0 upgrade and restarts is user-confirmed. Title counts may represent conversations.",
    },
    Platform {
        id: "telegram",
        name: "Telegram",
        entry_url: "https://web.telegram.org/a/",
        origin: "https://web.telegram.org",
        symbol: "TG",
        status: "experimental",
        available: true,
        limitations: "Authentication across three restarts is user-confirmed. Multiple Telegram identities, media and notifications still need independent acceptance. Native notification activity only.",
    },
    Platform {
        id: "discord",
        name: "Discord",
        entry_url: "https://discord.com/app",
        origin: "https://discord.com",
        symbol: "DC",
        status: "experimental",
        available: true,
        limitations: "Normal email/password or mobile QR login only. Embedded authentication, restart persistence, messages, media, voice and notifications need independent user acceptance. No numeric unread claims.",
    },
    Platform {
        id: "messenger",
        name: "Facebook Messenger",
        entry_url: "https://www.messenger.com/",
        origin: "https://www.messenger.com",
        symbol: "ME",
        status: "unavailable",
        available: false,
        limitations: "Proposed URL; not implemented or validated.",
    },
    Platform {
        id: "instagram",
        name: "Instagram Direct",
        entry_url: "https://www.instagram.com/direct/inbox/",
        origin: "https://www.instagram.com",
        symbol: "IG",
        status: "unavailable",
        available: false,
        limitations: "Proposed URL; not implemented or validated.",
    },
    Platform {
        id: "slack",
        name: "Slack",
        entry_url: "https://app.slack.com/",
        origin: "https://app.slack.com",
        symbol: "SL",
        status: "unavailable",
        available: false,
        limitations: "Workspace and SSO authentication review pending.",
    },
    Platform {
        id: "teams",
        name: "Microsoft Teams",
        entry_url: "https://teams.microsoft.com/",
        origin: "https://teams.microsoft.com",
        symbol: "MS",
        status: "unavailable",
        available: false,
        limitations: "Microsoft embedded authentication review pending.",
    },
    Platform {
        id: "gmail",
        name: "Gmail",
        entry_url: "https://mail.google.com/",
        origin: "https://mail.google.com",
        symbol: "GM",
        status: "unavailable",
        available: false,
        limitations: "Google embedded authentication restrictions require review.",
    },
    Platform {
        id: "custom",
        name: "Custom Website",
        entry_url: "",
        origin: "",
        symbol: "WEB",
        status: "unavailable",
        available: false,
        limitations: "Custom origin and redirect policy not yet implemented.",
    },
];
pub fn get(id: &str) -> Option<&'static Platform> {
    REGISTRY.iter().find(|p| p.id == id)
}
pub fn available(id: &str) -> Result<&'static Platform, String> {
    get(id)
        .filter(|p| p.available)
        .ok_or_else(|| "This platform is not available in this validation stage.".into())
}
impl Platform {
    pub fn navigation(&self, url: &Url) -> bool {
        !self.origin.is_empty()
            && url.scheme() == "https"
            && url.port_or_known_default() == Some(443)
            && url.username().is_empty()
            && url.password().is_none()
            && url.origin().ascii_serialization() == self.origin
    }
    pub fn permission(&self, origin: &str, kind: &str) -> bool {
        self.available
            && origin == self.origin
            && crate::desktop_policy::PERMISSIONS.contains(&kind)
    }
    pub fn unread(&self, title: &str) -> Option<crate::desktop_policy::Unread> {
        // Unvalidated provider title grammars never interpret chat names or invent counts.
        if self.id == "whatsapp" {
            crate::desktop_policy::unread_from_title(title)
        } else {
            None
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn discord_uses_only_official_https_navigation_and_separate_permissions() {
        let dc = available("discord").unwrap();
        assert_eq!(dc.status, "experimental");
        for path in ["/app", "/login", "/channels/@me", "/oauth2/authorize"] {
            assert!(dc.navigation(&Url::parse(&format!("https://discord.com{path}")).unwrap()));
        }
        for url in [
            "https://discord.com.evil.test/login",
            "https://discordapp.com/login",
            "http://discord.com/app",
            "https://user:password@discord.com/login",
            "https://discord.com:8443/login",
            "https://accounts.google.com/",
            "discord://login",
        ] {
            assert!(!dc.navigation(&Url::parse(url).unwrap()));
        }
        assert!(!dc.permission("https://web.telegram.org", "notifications"));
        assert!(!dc.permission("https://web.whatsapp.com", "microphone"));
        assert!(dc.permission(dc.origin, "notifications"));
        assert!(!dc.permission(dc.origin, "filesystem"));
        assert!(dc.unread("(99) Secret channel name").is_none());
        assert!(available("messenger").is_err());
    }
    #[test]
    fn exact_origins_and_no_authentication_bypass() {
        let wa = available("whatsapp").unwrap();
        let tg = available("telegram").unwrap();
        for bad in [
            "http://web.telegram.org/",
            "https://web.telegram.org.evil.test/",
            "https://user@web.telegram.org/",
            "https://web.telegram.org:444/",
            "https://telegram.org/",
            "javascript:alert(1)",
        ] {
            assert!(!tg.navigation(&Url::parse(bad).unwrap()));
        }
        assert!(tg.navigation(&Url::parse("https://web.telegram.org/k/#/login").unwrap()));
        assert!(!wa.navigation(&Url::parse(tg.entry_url).unwrap()));
        assert!(!tg.permission(wa.origin, "notifications"));
        assert!(!tg.permission(tg.origin, "filesystem"));
        assert!(tg.unread("(42) Private chat name").is_none());
        assert!(available("gmail").is_err());
        let ids: std::collections::HashSet<_> = REGISTRY.iter().map(|p| p.id).collect();
        assert_eq!(ids.len(), REGISTRY.len());
    }
}
