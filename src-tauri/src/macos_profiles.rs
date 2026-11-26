//! Persistent WKWebView stores. Never allow Wry's pre-macOS-14 shared-store fallback.
use objc2_foundation::{NSProcessInfo, NSString};
use objc2_web_kit::WKWebView;
use tauri::Manager;
use tauri::{Webview, webview::WebviewBuilder};

pub fn startup_url(url: &tauri::Url) -> bool {
    url.username().is_empty()
        && url.password().is_none()
        && url.query().is_none()
        && url.fragment().is_none()
        && url.path() == "/account-start.html"
        && ((url.scheme() == "tauri"
            && url.host_str() == Some("localhost")
            && url.port().is_none())
            || (cfg!(debug_assertions)
                && url.scheme() == "http"
                && url.host_str() == Some("localhost")
                && url.port() == Some(1420)))
}

/// DOM rectangles start inside WKWebView's safe content area, whereas child
/// frames are positioned in the containing NSView. Convert between those spaces.
pub fn set_bounds(
    app: &tauri::AppHandle,
    child: &Webview,
    bounds: crate::probe::Bounds,
) -> Result<(), String> {
    let shell = app.get_webview("main").ok_or("Main shell unavailable.")?;
    let child = child.clone();
    shell
        .with_webview(move |native| {
            // SAFETY: the native view is borrowed only on the UI thread.
            let frame = unsafe {
                let shell = &*native.inner().cast::<WKWebView>();
                let insets = shell.safeAreaInsets();
                let shell_bounds = shell.bounds();
                let y = if shell.isFlipped() {
                    shell_bounds.origin.y + insets.top + bounds.y
                } else {
                    shell_bounds.origin.y + shell_bounds.size.height
                        - insets.top
                        - bounds.y
                        - bounds.height
                };
                let rect = objc2_foundation::NSRect::new(
                    objc2_foundation::NSPoint::new(
                        shell_bounds.origin.x + insets.left + bounds.x,
                        y,
                    ),
                    objc2_foundation::NSSize::new(bounds.width, bounds.height),
                );
                shell.convertRect_toView(rect, shell.superview().as_deref())
            };
            let _ = child.with_webview(move |native| {
                // SAFETY: Tauri supplies the live child WKWebView on the UI thread.
                unsafe {
                    (&*native.inner().cast::<WKWebView>()).setFrame(frame);
                }
            });
        })
        .map_err(|_| "Could not align the account browser.".into())
}

pub fn identifier(id: &str) -> Result<[u8; 16], String> {
    let uuid = uuid::Uuid::parse_str(id).map_err(|_| "Invalid account UUID.")?;
    if uuid.is_nil() {
        return Err("A nonzero account UUID is required for persistent storage.".into());
    }
    Ok(uuid.into_bytes())
}

pub fn configure(
    builder: WebviewBuilder<tauri::Wry>,
    id: &str,
) -> Result<WebviewBuilder<tauri::Wry>, String> {
    if NSProcessInfo::processInfo()
        .operatingSystemVersion()
        .majorVersion
        < 14
    {
        return Err("Independent account storage requires macOS 14 or newer.".into());
    }
    Ok(builder
        .data_store_identifier(identifier(id)?)
        .incognito(false))
}

fn compatible_identity(native: &str, safari_version: &str) -> Result<String, String> {
    // Read the installed version; never invent a newer browser/engine version.
    if safari_version.len() > 32
        || safari_version
            .split('.')
            .any(|part| part.is_empty() || !part.bytes().all(|b| b.is_ascii_digit()))
    {
        return Err("Cannot determine the installed Safari compatibility version.".into());
    }
    let engine = native
        .split_whitespace()
        .find_map(|token| token.strip_prefix("AppleWebKit/"))
        .ok_or("Native WebKit identity unavailable.")?;
    if native.len() > 1024
        || engine.is_empty()
        || !engine.bytes().all(|b| b.is_ascii_digit() || b == b'.')
    {
        return Err("Unexpected native WebKit identity.".into());
    }
    if native
        .split_whitespace()
        .any(|token| token.starts_with("Version/"))
    {
        return Ok(format!(
            "{native} AppteriskMessenger/{}",
            env!("CARGO_PKG_VERSION")
        ));
    }
    Ok(format!(
        "{native} Version/{safari_version} Safari/{engine} AppteriskMessenger/{}",
        env!("CARGO_PKG_VERSION")
    ))
}

/// Configure compatibility tokens while the new account is on an empty local page.
/// No provider page, cookie, authentication value or stored browser data is read.
pub async fn prepare_whatsapp(view: &Webview) -> Result<(), String> {
    let metadata = plist::Value::from_file("/Applications/Safari.app/Contents/Info.plist")
        .map_err(
            |_| "Cannot read the installed Safari version. Install Safari before opening WhatsApp.",
        )?;
    let version = metadata
        .as_dictionary()
        .and_then(|d| d.get("CFBundleShortVersionString"))
        .and_then(plist::Value::as_string)
        .ok_or("Installed Safari version unavailable.")?;
    // A newly created WKWebView may still be committing its initial blank document.
    let start = std::time::Instant::now();
    let native = loop {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let tx = std::sync::Mutex::new(Some(tx));
        view.eval_with_callback("navigator.userAgent", move |value| {
            if let Some(tx) = tx.lock().unwrap().take() {
                let _ = tx.send(value);
            }
        })
        .map_err(|_| "Cannot inspect native browser identity.")?;
        if let Ok(Ok(value)) = tokio::time::timeout(std::time::Duration::from_millis(500), rx).await
        {
            if let Ok(native) = serde_json::from_str::<String>(&value) {
                break native;
            }
        }
        if start.elapsed() > std::time::Duration::from_secs(10) {
            return Err("Browser identity inspection timed out.".into());
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    };
    let identity = compatible_identity(&native, version)?;
    let (tx, rx) = tokio::sync::oneshot::channel();
    view.with_webview(move |native| {
        // SAFETY: the supplied WKWebView is live and this executes on its UI thread.
        unsafe {
            (&*native.inner().cast::<WKWebView>())
                .setCustomUserAgent(Some(&NSString::from_str(&identity)));
        }
        let _ = tx.send(());
    })
    .map_err(|_| "Could not set the native browser compatibility identity.")?;
    tokio::time::timeout(std::time::Duration::from_secs(10), rx)
        .await
        .map_err(|_| "Browser identity setup timed out.")?
        .map_err(|_| "Browser identity setup failed.".into())
}

pub async fn verify(view: &Webview, id: &str) -> Result<(), String> {
    let expected = identifier(id)?;
    let (tx, rx) = tokio::sync::oneshot::channel();
    view.with_webview(move |native| {
        // SAFETY: Tauri runs this closure on the UI thread and supplies the live WKWebView.
        let valid = unsafe {
            let webview = &*native.inner().cast::<WKWebView>();
            let store = webview.configuration().websiteDataStore();
            store.isPersistent()
                && store
                    .identifier()
                    .is_some_and(|id| id.as_bytes() == expected)
        };
        let _ = tx.send(valid);
    })
    .map_err(|_| "Cannot inspect native account storage.")?;
    match tokio::time::timeout(std::time::Duration::from_secs(10), rx).await {
        Ok(Ok(true)) => Ok(()),
        _ => Err("WKWebView did not confirm independent persistent account storage.".into()),
    }
}

#[cfg(debug_assertions)]
pub async fn geometry(view: &Webview) -> serde_json::Value {
    let (tx, rx) = tokio::sync::oneshot::channel();
    let _ = view.with_webview(move |native| {
        let data = unsafe {
            let view = &*native.inner().cast::<WKWebView>();
            let parent = view.superview().unwrap();
            let frame = view.frame();
            let top = if parent.isFlipped() { frame.origin.y } else { parent.bounds().size.height - frame.origin.y - frame.size.height };
            serde_json::json!({"x":frame.origin.x,"y":top,"width":frame.size.width,"height":frame.size.height,"insetTop":view.safeAreaInsets().top,"insetLeft":view.safeAreaInsets().left})
        };
        let _ = tx.send(data);
    });
    rx.await.unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_the_exact_empty_startup_asset_is_allowed() {
        assert!(startup_url(
            &"tauri://localhost/account-start.html".parse().unwrap()
        ));
        for url in [
            "tauri://localhost/index.html",
            "tauri://user@localhost/account-start.html",
            "https://localhost/account-start.html",
            "tauri://localhost/account-start.html?remote=true",
        ] {
            assert!(!startup_url(&url.parse().unwrap()));
        }
    }
    #[test]
    fn compatibility_preserves_native_engine_and_validates_version() {
        let native = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko)";
        let ua = compatible_identity(native, "27.2").unwrap();
        assert!(ua.starts_with(native));
        assert!(ua.contains("Version/27.2 Safari/605.1.15 AppteriskMessenger/"));
        for version in ["", "99.fake", "27\nChrome/999", ".27", "27."] {
            assert!(compatible_identity(native, version).is_err());
        }
        assert!(compatible_identity("Chrome/999", "27.2").is_err());
    }
    #[test]
    fn store_identity_is_stable_distinct_and_requires_uuid() {
        let a = "c4b9dc51-80ce-4e57-a738-4c182a216b01";
        let b = "c4b9dc51-80ce-4e57-a738-4c182a216b02";
        assert_eq!(
            identifier(a).unwrap(),
            uuid::Uuid::parse_str(a).unwrap().into_bytes()
        );
        assert_ne!(identifier(a).unwrap(), identifier(b).unwrap());
        assert!(identifier("invalid").is_err());
        assert!(identifier("00000000-0000-0000-0000-000000000000").is_err());
    }
}
