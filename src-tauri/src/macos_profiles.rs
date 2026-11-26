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
