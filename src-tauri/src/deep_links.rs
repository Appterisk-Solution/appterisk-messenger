//! Untrusted links are parsed into a small set of chat destinations, never commands.
use crate::{accounts, database, desktop, probe::Bounds};
use std::sync::Mutex;
use tauri::{Emitter, Manager, Webview};
type Result<T> = std::result::Result<T, String>;

#[derive(Clone, Debug, PartialEq)]
enum Destination {
    WhatsApp {
        phone: String,
        text: Option<String>,
    },
    Telegram {
        username: String,
        post: Option<String>,
    },
}
impl Destination {
    fn platform(&self) -> &'static str {
        match self {
            Self::WhatsApp { .. } => "whatsapp",
            Self::Telegram { .. } => "telegram",
        }
    }
    fn web_url(&self, telegram_entry: &str) -> tauri::Url {
        match self {
            Self::WhatsApp { phone, text } => {
                let mut url = tauri::Url::parse("https://web.whatsapp.com/send").unwrap();
                url.query_pairs_mut().append_pair("phone", phone);
                if let Some(text) = text {
                    url.query_pairs_mut().append_pair("text", text);
                }
                url
            }
            Self::Telegram { username, post } => {
                let mut target = tauri::Url::parse("tg://resolve").unwrap();
                target.query_pairs_mut().append_pair("domain", username);
                if let Some(post) = post {
                    target.query_pairs_mut().append_pair("post", post);
                }
                let mut url = tauri::Url::parse(telegram_entry).unwrap();
                url.set_fragment(Some(&format!(
                    "?{}",
                    tauri::Url::parse_with_params(
                        "https://unused.invalid/",
                        [("tgaddr", target.as_str())]
                    )
                    .unwrap()
                    .query()
                    .unwrap()
                )));
                url
            }
        }
    }
}
fn parse(raw: &str) -> Option<Destination> {
    if raw.len() > 8192 || raw.chars().any(char::is_control) {
        return None;
    }
    let url = tauri::Url::parse(raw.trim()).ok()?;
    if !url.username().is_empty()
        || url.password().is_some()
        || url.port().is_some()
        || url.fragment().is_some()
    {
        return None;
    }
    let pairs: Vec<_> = url.query_pairs().collect();
    // Reject duplicate and unknown parameters, including authentication and proxy payloads.
    if pairs
        .iter()
        .enumerate()
        .any(|(i, (k, _))| pairs[..i].iter().any(|(other, _)| other == k))
    {
        return None;
    }
    let value = |key: &str| {
        pairs
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.to_string())
    };
    let host = url.host_str()?;
    let whatsapp = (url.scheme() == "whatsapp" && host == "send" && matches!(url.path(), "" | "/"))
        || (url.scheme() == "https" && host == "api.whatsapp.com" && url.path() == "/send");
    if whatsapp || (url.scheme() == "https" && host == "wa.me") {
        if pairs
            .iter()
            .any(|(k, _)| !matches!(k.as_ref(), "phone" | "text"))
        {
            return None;
        }
        let phone = if whatsapp {
            value("phone")?
        } else {
            if value("phone").is_some() {
                return None;
            }
            url.path().trim_start_matches('/').to_string()
        };
        let phone = phone.strip_prefix('+').unwrap_or(&phone).to_string();
        if !(7..=15).contains(&phone.len()) || !phone.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        let text = value("text");
        if text.as_ref().is_some_and(|v| {
            v.len() > 4096
                || v.chars()
                    .any(|c| c.is_control() && !matches!(c, '\n' | '\r' | '\t'))
        }) {
            return None;
        }
        return Some(Destination::WhatsApp { phone, text });
    }
    let (username, post) = if url.scheme() == "tg"
        && host == "resolve"
        && matches!(url.path(), "" | "/")
    {
        if pairs
            .iter()
            .any(|(k, _)| !matches!(k.as_ref(), "domain" | "post"))
        {
            return None;
        }
        (value("domain")?, value("post"))
    } else if url.scheme() == "https" && matches!(host, "t.me" | "telegram.me") && pairs.is_empty()
    {
        let parts: Vec<_> = url.path().trim_matches('/').split('/').collect();
        if parts.is_empty() || parts.len() > 2 {
            return None;
        }
        (parts[0].to_string(), parts.get(1).map(|s| s.to_string()))
    } else {
        return None;
    };
    if username.is_empty()
        || username.len() > 32
        || !username.as_bytes()[0].is_ascii_alphabetic()
        || !username
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_')
        || [
            "login",
            "proxy",
            "socks",
            "share",
            "joinchat",
            "addstickers",
            "addemoji",
            "addlist",
            "setlanguage",
            "confirmphone",
            "invoice",
            "giftcode",
            "boost",
            "contact",
            "auth",
        ]
        .contains(&username.as_str())
        || post.as_ref().is_some_and(|p| {
            p.is_empty()
                || p.len() > 20
                || p.starts_with('0')
                || !p.bytes().all(|b| b.is_ascii_digit())
        })
    {
        return None;
    }
    Some(Destination::Telegram { username, post })
}
#[derive(Clone)]
struct Request {
    id: String,
    destination: Destination,
}
#[derive(serde::Serialize)]
pub struct Pending {
    id: String,
    platform: &'static str,
}
pub struct LinkState(Mutex<Option<Request>>);
impl LinkState {
    pub fn from_args() -> Self {
        let destination = std::env::args().skip(1).find_map(|a| parse(&a));
        Self(Mutex::new(destination.map(|destination| Request {
            id: uuid::Uuid::new_v4().to_string(),
            destination,
        })))
    }
}
pub fn incoming(app: &tauri::AppHandle, raw: &str) -> bool {
    let Some(destination) = parse(raw) else {
        return false;
    };
    let Some(state) = app.try_state::<LinkState>() else {
        return false;
    };
    let Ok(mut pending) = state.0.lock() else {
        return false;
    };
    *pending = Some(Request {
        id: uuid::Uuid::new_v4().to_string(),
        destination,
    });
    drop(pending);
    desktop::restore(app, false);
    let _ = app.emit_to("main", "chat-link-pending", ());
    true
}
#[tauri::command]
pub fn chat_link_pending(app: tauri::AppHandle, caller: Webview) -> Result<Option<Pending>> {
    desktop::authorized(&caller)?;
    if desktop::locked(&app) {
        return Ok(None);
    }
    let state = app.state::<LinkState>();
    let pending = state.0.lock().map_err(|_| "Link state unavailable.")?;
    Ok(pending.as_ref().map(|r| Pending {
        id: r.id.clone(),
        platform: r.destination.platform(),
    }))
}
#[tauri::command]
pub fn chat_link_submit(app: tauri::AppHandle, caller: Webview, url: String) -> Result<()> {
    desktop::authorized(&caller)?;
    if desktop::locked(&app) {
        return Err("Unlock Appterisk before opening a link.".into());
    }
    if incoming(&app, &url) {
        Ok(())
    } else {
        Err("Unsupported link. Use a WhatsApp phone/chat link or Telegram username/public-message link.".into())
    }
}
#[tauri::command]
pub async fn chat_link_open(
    app: tauri::AppHandle,
    caller: Webview,
    request_id: String,
    account_id: Option<String>,
    bounds: Bounds,
) -> Result<()> {
    desktop::authorized(&caller)?;
    if crate::health::closing() || accounts::diagnostic_mode() {
        return Err(
            "Chat links are unavailable while closing or in compatibility diagnostic mode.".into(),
        );
    }
    if desktop::locked(&app) {
        return Err("Unlock Appterisk before opening a link.".into());
    }
    let request = {
        let state = app.state::<LinkState>();
        let pending = state.0.lock().map_err(|_| "Link state unavailable.")?;
        pending
            .as_ref()
            .filter(|r| r.id == request_id)
            .cloned()
            .ok_or("This link was replaced. Choose the account again.")?
    };
    if let Some(id) = account_id {
        let state = app.state::<accounts::AccountState>();
        let mut runtime = state.0.lock().await;
        if crate::health::closing() || accounts::diagnostic_mode() {
            return Err(
                "Chat links are unavailable while closing or in compatibility diagnostic mode."
                    .into(),
            );
        }
        if desktop::locked(&app) {
            return Err("Unlock Appterisk before opening a link.".into());
        }
        let account = runtime.db.account(&id)?;
        if account.platform != request.destination.platform() {
            return Err("Choose an account for this service.".into());
        }
        accounts::switch(&app, &mut runtime, &id, bounds).await?;
        let entry = if account.platform == "telegram" {
            crate::telegram_client::entry_url(&database::profile_path(&runtime.root, &account)?)?
        } else {
            ""
        };
        let url = request.destination.web_url(entry);
        if !crate::platforms::available(&account.platform)?.navigation(&url) {
            return Err("Untrusted chat destination.".into());
        }
        app.get_webview(&accounts::label(&id))
            .ok_or("Account browser unavailable.")?
            .navigate(url)
            .map_err(|_| "Could not open chat link.")?;
    }
    let state = app.state::<LinkState>();
    let mut pending = state.0.lock().map_err(|_| "Link state unavailable.")?;
    if pending.as_ref().is_some_and(|r| r.id == request_id) {
        *pending = None;
    }
    Ok(())
}
#[tauri::command]
pub fn chat_link_defaults(app: tauri::AppHandle, caller: Webview) -> Result<()> {
    desktop::authorized(&caller)?;
    if desktop::locked(&app) {
        return Err("Unlock Appterisk first.".into());
    }
    #[cfg(windows)]
    {
        let _ = app;
        open::that_detached("ms-settings:defaultapps")
            .map_err(|_| "Could not open Windows Default apps.".into())
    }
    #[cfg(not(windows))]
    Err("Link association settings are currently available on Windows only.".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn common_links_map_only_to_official_web_destinations() {
        for link in [
            "whatsapp://send?phone=60123456789&text=Hello%20there",
            "https://wa.me/60123456789?text=Hello%20there",
        ] {
            let target = parse(link).unwrap();
            let url = target.web_url("");
            assert_eq!(
                url.origin().ascii_serialization(),
                "https://web.whatsapp.com"
            );
            assert_eq!(
                url.query_pairs().find(|(k, _)| k == "text").unwrap().1,
                "Hello there"
            );
        }
        let target = parse("tg://resolve?domain=telegram&post=42").unwrap();
        assert_eq!(target, parse("https://t.me/telegram/42").unwrap());
        for entry in ["https://web.telegram.org/a/", "https://web.telegram.org/k/"] {
            let url = target.web_url(entry);
            assert_eq!(
                url.origin().ascii_serialization(),
                "https://web.telegram.org"
            );
            assert!(
                url.fragment()
                    .unwrap()
                    .starts_with("?tgaddr=tg%3A%2F%2Fresolve")
            );
        }
    }
    #[test]
    fn rejects_commands_authentication_and_ambiguous_links() {
        for link in [
            "file:///secret",
            "javascript:alert(1)",
            "https://wa.me.evil.test/60123456789",
            "https://user@wa.me/60123456789",
            "https://wa.me:444/60123456789",
            "whatsapp://send?phone=60123456789&phone=60123456788",
            "whatsapp://send?phone=60123456789&command=quit",
            "tg://login?token=secret",
            "tg://proxy?server=evil",
            "tg://resolve?domain=telegram&start=send",
            "https://t.me/login",
            "https://t.me/user/../../login",
            "https://t.me/telegram?tgWebAuthToken=secret",
            "appterisk://settings/quit",
        ] {
            assert!(parse(link).is_none(), "{link}");
        }
        assert!(parse(&"x".repeat(8193)).is_none());
    }
}
