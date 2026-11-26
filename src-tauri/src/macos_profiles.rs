//! Persistent WKWebView stores. Never allow Wry's pre-macOS-14 shared-store fallback.
use objc2_foundation::NSProcessInfo;
use objc2_web_kit::WKWebView;
use tauri::Manager;
use tauri::{Webview, webview::WebviewBuilder};

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
        return Ok(native.to_owned());
    }
    Ok(format!("{native} Version/{safari_version} Safari/{engine}"))
}

/// Read compatibility identity from the trusted shell before constructing an account view.
/// No provider page, cookie, authentication value or stored browser data is read.
pub async fn whatsapp_identity(view: &Webview) -> Result<String, String> {
    if view.label() != "main" {
        return Err("Browser identity must come from the trusted shell.".into());
    }
    let metadata = plist::Value::from_file("/Applications/Safari.app/Contents/Info.plist")
        .map_err(
            |_| "Cannot read the installed Safari version. Install Safari before opening WhatsApp.",
        )?;
    let version = metadata
        .as_dictionary()
        .and_then(|d| d.get("CFBundleShortVersionString"))
        .and_then(plist::Value::as_string)
        .ok_or("Installed Safari version unavailable.")?;
    // Startup diagnostics can run while the trusted shell is still loading.
    let start = std::time::Instant::now();
    let native = loop {
        if crate::probe::trusted(view).is_err() {
            if start.elapsed() > std::time::Duration::from_secs(10) {
                return Err("Trusted shell is not ready for browser identity setup.".into());
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            continue;
        }
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
    compatible_identity(&native, version)
}

pub async fn verify_identity(view: &Webview, expected: &str) -> Result<(), String> {
    let expected = expected.to_owned();
    let (tx, rx) = tokio::sync::oneshot::channel();
    view.with_webview(move |native| {
        // SAFETY: Tauri supplies a live WKWebView on its UI thread.
        let matches = unsafe {
            (&*native.inner().cast::<WKWebView>())
                .customUserAgent()
                .is_some_and(|identity| identity.to_string() == expected)
        };
        let _ = tx.send(matches);
    })
    .map_err(|_| "Cannot verify the native browser identity.")?;
    match tokio::time::timeout(std::time::Duration::from_secs(10), rx).await {
        Ok(Ok(true)) => Ok(()),
        _ => Err("Native account browser identity verification failed.".into()),
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
struct CapabilitySnapshot {
    secure: bool,
    crypto: bool,
    indexeddb: bool,
    worker: bool,
    serviceworker: bool,
    controlled: bool,
    notification: bool,
    wasm: bool,
    sharedarray: bool,
    errors: Vec<DiagnosticKind>,
}
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
enum DiagnosticKind {
    Clone,
    Quota,
    Crypto,
    Indexeddb,
    Worker,
    Notification,
    Network,
    Unsupported,
    Type,
    Other,
}

pub fn observe_capabilities(view: &Webview, id: &str) {
    let view = view.clone();
    let id = id.to_owned();
    tauri::async_runtime::spawn(async move {
        let mut previous = String::new();
        for _ in 0..5 {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            let (tx, rx) = tokio::sync::oneshot::channel();
            let tx = std::sync::Mutex::new(Some(tx));
            if view
                .eval_with_callback(
                    "JSON.stringify(window.__appteriskCapabilitySnapshot?.()??null)",
                    move |value| {
                        if let Some(tx) = tx.lock().unwrap().take() {
                            let _ = tx.send(value);
                        }
                    },
                )
                .is_err()
            {
                return;
            }
            if let Ok(Ok(value)) = tokio::time::timeout(std::time::Duration::from_secs(2), rx).await
            {
                if value.len() > 4096 || value == previous {
                    continue;
                }
                if let Ok(encoded) = serde_json::from_str::<String>(&value) {
                    if let Ok(report) = serde_json::from_str::<CapabilitySnapshot>(&encoded) {
                        if report.errors.len() <= 20 {
                            crate::health::log(
                                view.app_handle(),
                                "macos_capability_snapshot",
                                serde_json::json!({"profile_id":id,"app_version":env!("CARGO_PKG_VERSION"),"report":report}),
                            );
                            previous = value;
                        }
                    }
                }
            }
        }
    });
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
    fn capability_diagnostics_reject_private_text_and_unknown_categories() {
        let flags = serde_json::json!({"secure":true,"crypto":true,"indexeddb":true,"worker":true,"serviceworker":true,"controlled":false,"notification":false,"wasm":true,"sharedarray":false,"errors":["crypto"]});
        assert!(serde_json::from_value::<CapabilitySnapshot>(flags.clone()).is_ok());
        let mut private = flags.clone();
        private["message"] = serde_json::json!("must never be collected");
        assert!(serde_json::from_value::<CapabilitySnapshot>(private).is_err());
        let mut unknown = flags;
        unknown["errors"] = serde_json::json!(["arbitrary private text"]);
        assert!(serde_json::from_value::<CapabilitySnapshot>(unknown).is_err());
    }
    #[test]
    fn compatibility_preserves_native_engine_and_validates_version() {
        let native = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko)";
        let ua = compatible_identity(native, "27.2").unwrap();
        assert!(ua.starts_with(native));
        assert_eq!(ua, format!("{native} Version/27.2 Safari/605.1.15"));
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
