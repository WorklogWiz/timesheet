//! System monitoring (sleep/wake detection)
//!
//! Uses macOS `NSWorkspace` notifications to detect system sleep and wake events.

// Suppress warnings from cocoa/objc crates (external dependencies)
// TODO: Migrate to objc2/objc2-foundation/objc2-app-kit when ready
#![allow(unexpected_cfgs)]
#![allow(deprecated)]

use chrono::{DateTime, Utc};
use std::sync::{Arc, Mutex};
#[cfg(target_os = "macos")]
use std::thread;
use tauri::AppHandle;
#[cfg(target_os = "macos")]
use tauri::{Emitter, Manager};
#[cfg(target_os = "macos")]
use tauri_plugin_notification::NotificationExt;

#[cfg(target_os = "macos")]
use block::ConcreteBlock;
#[cfg(target_os = "macos")]
use cocoa::base::{id, nil};
#[cfg(target_os = "macos")]
use cocoa::foundation::NSString;
#[cfg(target_os = "macos")]
use objc::{class, msg_send, sel, sel_impl};

pub struct SystemMonitors {
    sleep_monitor: Arc<Mutex<SleepMonitor>>,
}

impl SystemMonitors {
    pub fn new() -> Self {
        Self {
            sleep_monitor: Arc::new(Mutex::new(SleepMonitor::new())),
        }
    }

    #[cfg(target_os = "macos")]
    pub fn start(&self, app: &AppHandle) {
        let monitor = self.sleep_monitor.clone();
        let app_clone = app.clone();
        thread::spawn(move || {
            monitor.lock().unwrap().start_monitoring(&app_clone);
        });
    }

    #[cfg(not(target_os = "macos"))]
    #[allow(clippy::unused_self)]
    pub fn start(&self, _app: &AppHandle) {
        // No-op on non-macOS platforms
    }

    #[allow(dead_code)]
    pub fn did_wake(&self) -> bool {
        self.sleep_monitor.lock().unwrap().did_wake
    }

    #[allow(dead_code)]
    pub fn reset_wake_status(&self) {
        self.sleep_monitor.lock().unwrap().did_wake = false;
    }
}

pub struct SleepMonitor {
    #[allow(dead_code)]
    pub did_wake: bool,
    #[allow(dead_code)]
    pub sleep_time: Arc<Mutex<Option<DateTime<Utc>>>>,
}

impl SleepMonitor {
    pub fn new() -> Self {
        Self {
            did_wake: false,
            sleep_time: Arc::new(Mutex::new(None)),
        }
    }

    #[cfg(target_os = "macos")]
    #[allow(clippy::too_many_lines)] // Complex sleep/wake monitoring logic
    pub fn start_monitoring(&mut self, app: &AppHandle) {
        eprintln!(
            "🛌 Sleep monitor started - registering for macOS system sleep/wake notifications"
        );

        let sleep_time = self.sleep_time.clone();

        // Use NSWorkspace notifications for system sleep/wake
        unsafe {
            // Get shared workspace
            let workspace_class = class!(NSWorkspace);
            let workspace: id = msg_send![workspace_class, sharedWorkspace];

            // Get notification center
            let notification_center: id = msg_send![workspace, notificationCenter];

            // NSWorkspaceWillSleepNotification
            let sleep_notif_name =
                NSString::alloc(nil).init_str("NSWorkspaceWillSleepNotification");

            // NSWorkspaceDidWakeNotification
            let wake_notif_name = NSString::alloc(nil).init_str("NSWorkspaceDidWakeNotification");

            // Register for sleep notification
            let sleep_time_for_sleep = sleep_time.clone();
            let sleep_block = ConcreteBlock::new(move |_notification: id| {
                let now = Utc::now();
                *sleep_time_for_sleep.lock().unwrap() = Some(now);
                eprintln!(
                    "💤 System going to sleep at {}",
                    now.format("%Y-%m-%d %H:%M:%S UTC")
                );
            });
            let sleep_block = sleep_block.copy();

            let _: () = msg_send![notification_center,
                addObserverForName:sleep_notif_name
                object:nil
                queue:nil
                usingBlock:&*sleep_block
            ];

            // Register for wake notification
            let app_for_wake = app.clone();
            let sleep_time_for_wake = sleep_time.clone();
            let wake_block = ConcreteBlock::new(move |_notification: id| {
                let wake_time = Utc::now();
                eprintln!(
                    "⏰ System woke up at {}",
                    wake_time.format("%Y-%m-%d %H:%M:%S UTC")
                );

                // Calculate sleep duration
                if let Some(sleep_start) = *sleep_time_for_wake.lock().unwrap() {
                    let duration = wake_time.signed_duration_since(sleep_start);
                    let seconds = u64::try_from(duration.num_seconds().max(0)).unwrap_or(0);

                    let hours = seconds / 3600;
                    let minutes = (seconds % 3600) / 60;

                    eprintln!("💤 System was asleep for {hours}h {minutes}m");

                    // Check if there's an active timer by trying to get the timesheet integration
                    let has_active_timer = if let Some(timesheet) = app_for_wake
                        .try_state::<Arc<crate::timesheet_integration::TimesheetIntegration>>()
                    {
                        // Use blocking call to check for active timer
                        let runtime = tokio::runtime::Runtime::new().unwrap();
                        runtime.block_on(async {
                            timesheet.get_active_timer().await.ok().flatten().is_some()
                        })
                    } else {
                        false
                    };

                    if has_active_timer {
                        eprintln!("🏃 Active timer detected - showing notification");

                        // Format duration text
                        let duration_text = if hours > 0 {
                            format!("{hours}h {minutes}m")
                        } else {
                            format!("{minutes}m")
                        };

                        // Convert sleep_start (DateTime<Local>) to UTC
                        let sleep_start_utc = sleep_start.with_timezone(&chrono::Utc);

                        // Emit event to signal sleep wake (for menu update)
                        let payload = serde_json::json!({
                            "sleepDuration": duration_text,
                            "sleepStartTime": sleep_start_utc.to_rfc3339(),
                            "sleepDurationSeconds": i64::try_from(seconds).unwrap_or(i64::MAX),
                        });

                        if let Err(e) = app_for_wake.emit("sleep-wake-detected", payload) {
                            eprintln!("❌ Failed to emit sleep-wake-detected event: {e}");
                        } else {
                            eprintln!("✅ Sleep wake event emitted");
                        }

                        // Show a notification directing user to menubar
                        let _ = app_for_wake
                            .notification()
                            .builder()
                            .title("💤 System Wake - Timer Active")
                            .body(format!(
                                "Your computer was asleep for {duration_text}.\n\nCheck the menubar to continue or stop your timer."
                            ))
                            .sound("default")
                            .show();
                    } else {
                        eprintln!("ℹ️ No active timer - skipping notification");
                    }

                    // Emit wake event to frontend (if any windows are open)
                    let result = app_for_wake.emit(
                        "system-wake",
                        serde_json::json!({
                            "sleep_duration_seconds": seconds
                        }),
                    );

                    if let Err(e) = result {
                        eprintln!("❌ Failed to emit system-wake event: {e}");
                    } else {
                        eprintln!("✅ Successfully emitted system-wake event to frontend");
                    }
                } else {
                    eprintln!("⚠️ Wake detected but no sleep time recorded");
                }

                // Clear sleep time
                *sleep_time_for_wake.lock().unwrap() = None;
            });
            let wake_block = wake_block.copy();

            let _: () = msg_send![notification_center,
                addObserverForName:wake_notif_name
                object:nil
                queue:nil
                usingBlock:&*wake_block
            ];

            eprintln!("✅ Sleep/wake notification observers registered");
        }

        // Keep thread alive
        loop {
            thread::sleep(std::time::Duration::from_secs(60));
        }
    }

    #[cfg(not(target_os = "macos"))]
    #[allow(dead_code)]
    #[allow(clippy::unused_self)]
    pub fn start_monitoring(&mut self, _app: &AppHandle) {
        eprintln!("⚠️ Sleep monitoring not available on this platform");
    }
}

pub struct CameraMonitor {
    #[allow(dead_code)]
    pub is_in_use: bool,
}

#[allow(dead_code)]
impl CameraMonitor {
    pub fn new() -> Self {
        Self { is_in_use: false }
    }

    #[cfg(target_os = "macos")]
    pub fn check_status(&mut self) -> bool {
        // Simplified camera monitoring
        // A complete implementation would use AVCaptureDevice
        self.is_in_use
    }

    #[cfg(not(target_os = "macos"))]
    #[allow(clippy::unused_self)]
    pub fn check_status(&mut self) -> bool {
        false
    }
}

pub struct MicrophoneMonitor {
    #[allow(dead_code)]
    pub is_in_use: bool,
}

#[allow(dead_code)]
impl MicrophoneMonitor {
    pub fn new() -> Self {
        Self { is_in_use: false }
    }

    #[cfg(target_os = "macos")]
    pub fn check_status(&mut self) -> bool {
        // Simplified microphone monitoring
        // A complete implementation would use AVCaptureDevice
        self.is_in_use
    }

    #[cfg(not(target_os = "macos"))]
    #[allow(clippy::unused_self)]
    pub fn check_status(&mut self) -> bool {
        false
    }
}
