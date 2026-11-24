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
        limitations: "Verified on v0.3.0; v0.4.0 upgrade acceptance pending. Title counts may represent conversations.",
    },
    Platform {
        id: "telegram",
        name: "Telegram",
        entry_url: "https://web.telegram.org/k/",
        origin: "https://web.telegram.org",
        symbol: "TG",
        status: "experimental",
        available: true,
        limitations: "Real authentication, restart persistence, media and notifications require independent user acceptance. Unread activity uses native notifications only.",
    },
    Platform {
        id: "discord",
        name: "Discord",
        entry_url: "https://discord.com/app",
        origin: "https://discord.com",
        symbol: "DC",
        status: "unavailable",
        available: false,
        limitations: "Awaiting Telegram validation and official authentication review.",
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
        // Telegram's title grammar has not been independently validated. Never interpret chat names.
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
