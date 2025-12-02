//! Window extensions for macOS-specific functionality

// Suppress warnings from cocoa crate (external dependency)
// TODO: Migrate to objc2-app-kit when ready
#![allow(deprecated)]

#[cfg(target_os = "macos")]
use cocoa::appkit::{NSWindow, NSWindowStyleMask, NSWindowTitleVisibility};
#[cfg(target_os = "macos")]
use cocoa::base::{id, NO, YES};
use tauri::{Runtime, WebviewWindow};

#[allow(dead_code)]
pub trait WindowExt {
    fn set_transparent_titlebar(&self, transparent: bool);
}

#[cfg(target_os = "macos")]
impl<R: Runtime> WindowExt for WebviewWindow<R> {
    fn set_transparent_titlebar(&self, transparent: bool) {
        unsafe {
            let ns_window = self.ns_window().unwrap() as id;

            let mut style_mask = ns_window.styleMask();
            style_mask.set(
                NSWindowStyleMask::NSFullSizeContentViewWindowMask,
                transparent,
            );
            ns_window.setStyleMask_(style_mask);

            ns_window.setTitleVisibility_(if transparent {
                NSWindowTitleVisibility::NSWindowTitleHidden
            } else {
                NSWindowTitleVisibility::NSWindowTitleVisible
            });

            ns_window.setTitlebarAppearsTransparent_(if transparent { YES } else { NO });
        }
    }
}

#[cfg(not(target_os = "macos"))]
impl<R: Runtime> WindowExt for WebviewWindow<R> {
    fn set_transparent_titlebar(&self, _transparent: bool) {
        // No-op on non-macOS platforms
    }
}
