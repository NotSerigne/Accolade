// src-tauri/src/overlay.rs
//! Keeps the achievement overlay above fullscreen games.
//!
//! Fullscreen games commonly mark their own window as topmost, which pushes a window that was
//! only made topmost once at startup underneath them. Re-asserting `HWND_TOPMOST` right before
//! showing something puts the overlay back at the top of the topmost band, without ever
//! activating it (stealing focus would minimise an exclusive-fullscreen game).
use tauri::{AppHandle, Manager, Runtime};

pub const OVERLAY_LABEL: &str = "achievement-overlay";

/// Shared WebView2 browser arguments. All webviews of the app share one WebView2 environment,
/// so the main window (tauri.conf.json → `additionalBrowserArgs`) must use the exact same string.
///
/// Besides wry's defaults, this disables Chromium's native window occlusion tracking and
/// background throttling: otherwise, as soon as a fullscreen game covers the overlay, the page
/// is considered hidden and `requestAnimationFrame`/timers are frozen, so notifications only
/// appear once the game is alt-tabbed.
pub const WEBVIEW_BROWSER_ARGS: &str = "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection,CalculateNativeWinOcclusion --disable-background-timer-throttling --disable-backgrounding-occluded-windows --disable-renderer-backgrounding";

/// Brings the overlay window back to the top of the z-order without activating it.
pub fn raise<R: Runtime>(app: &AppHandle<R>) {
    let Some(window) = app.get_webview_window(OVERLAY_LABEL) else {
        return;
    };

    #[cfg(target_os = "windows")]
    {
        use windows::Win32::Foundation::HWND;
        use windows::Win32::UI::WindowsAndMessaging::{
            SetWindowPos, HWND_TOPMOST, SWP_ASYNCWINDOWPOS, SWP_NOACTIVATE, SWP_NOMOVE,
            SWP_NOOWNERZORDER, SWP_NOSIZE, SWP_SHOWWINDOW,
        };

        match window.hwnd() {
            Ok(handle) => {
                let hwnd = HWND(handle.0 as _);
                let result = unsafe {
                    SetWindowPos(
                        hwnd,
                        Some(HWND_TOPMOST),
                        0,
                        0,
                        0,
                        0,
                        SWP_NOMOVE
                            | SWP_NOSIZE
                            | SWP_NOACTIVATE
                            | SWP_NOOWNERZORDER
                            | SWP_SHOWWINDOW
                            | SWP_ASYNCWINDOWPOS,
                    )
                };
                if let Err(e) = result {
                    log::warn!("[overlay] SetWindowPos(HWND_TOPMOST) failed: {}", e);
                }
            }
            Err(e) => log::warn!("[overlay] Could not get overlay HWND: {}", e),
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = window.set_always_on_top(true);
    }
}

#[tauri::command]
pub fn raise_overlay(app: AppHandle) {
    raise(&app);
}
