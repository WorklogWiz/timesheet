#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod models;
mod monitors;
mod services;
mod timesheet_integration;
mod window_ext;

use chrono::{DateTime, Utc};
use models::WorkSession;
use monitors::SystemMonitors;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Instant;
use tauri::image::Image;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{TrayIcon, TrayIconBuilder, TrayIconEvent};
#[cfg(target_os = "macos")]
use tauri::LogicalPosition;
use tauri::{AppHandle, Emitter, Listener, Manager, WebviewUrl, WebviewWindowBuilder};
use timesheet_integration::TimesheetIntegration;
#[cfg(target_os = "macos")]
use window_ext::WindowExt;

#[derive(Clone, serde::Serialize)]
struct TimerUpdatePayload {
    time: String,
    in_progress: bool,
}

// Simple timer state for UI updates
struct TimerState {
    start_time: Option<Instant>,
    start_date: Option<DateTime<Utc>>,
}

// Sleep wake state to track pending user response
struct SleepWakeState {
    awaiting_response: bool,
    sleep_duration: String,
    sleep_start_time: Option<DateTime<Utc>>,
    sleep_duration_seconds: i64,
}

impl SleepWakeState {
    fn new() -> Self {
        Self {
            awaiting_response: false,
            sleep_duration: String::new(),
            sleep_start_time: None,
            sleep_duration_seconds: 0,
        }
    }

    fn set_awaiting_response(
        &mut self,
        duration: String,
        start_time: DateTime<Utc>,
        duration_secs: i64,
    ) {
        self.awaiting_response = true;
        self.sleep_duration = duration;
        self.sleep_start_time = Some(start_time);
        self.sleep_duration_seconds = duration_secs;
    }

    fn clear(&mut self) {
        self.awaiting_response = false;
        self.sleep_duration.clear();
        self.sleep_start_time = None;
        self.sleep_duration_seconds = 0;
    }
}

impl TimerState {
    fn new() -> Self {
        Self {
            start_time: None,
            start_date: None,
        }
    }

    fn start(&mut self) {
        self.start_time = Some(Instant::now());
        self.start_date = Some(Utc::now());
    }

    fn restore_from(&mut self, start_date: DateTime<Utc>) {
        // Calculate how much time has elapsed since the original start
        let elapsed = Utc::now().signed_duration_since(start_date);
        let elapsed_secs = u64::try_from(elapsed.num_seconds().max(0)).unwrap_or(0);

        // Set start_time to an instant in the past
        self.start_time = Instant::now().checked_sub(std::time::Duration::from_secs(elapsed_secs));
        self.start_date = Some(start_date);
    }

    fn stop(&mut self) {
        self.start_time = None;
        self.start_date = None;
    }

    fn is_running(&self) -> bool {
        self.start_time.is_some()
    }

    fn get_elapsed_time(&self) -> String {
        if let Some(start) = self.start_time {
            let elapsed = start.elapsed().as_secs();
            let hours = elapsed / 3600;
            let minutes = (elapsed % 3600) / 60;
            let seconds = elapsed % 60;
            format!("{hours:02}:{minutes:02}:{seconds:02}")
        } else {
            "00:00:00".to_string()
        }
    }
}

// Tauri commands
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
async fn start_timer(
    timesheet: tauri::State<'_, Arc<TimesheetIntegration>>,
    timer_state: tauri::State<'_, Arc<Mutex<TimerState>>>,
    app: AppHandle,
    issue_key: Option<String>,
) -> Result<String, String> {
    // Start timer - issue_key is optional
    timesheet
        .start_timer(issue_key.as_deref(), None)
        .await
        .map_err(|e| e.to_string())?;

    // Update UI timer state
    timer_state.lock().unwrap().start();

    // Update tray icon
    update_tray_menu(&app, true);

    // Emit event to refresh views
    let _ = app.emit("timer-started", ());

    Ok("started".to_string())
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
async fn get_active_timer(
    timesheet: tauri::State<'_, Arc<TimesheetIntegration>>,
) -> Result<Option<WorkSession>, String> {
    timesheet
        .get_active_timer()
        .await
        .map_err(|e| format!("Failed to get active timer: {e}"))
}

#[tauri::command]
async fn stop_timer(
    timesheet: tauri::State<'_, Arc<TimesheetIntegration>>,
    timer_state: tauri::State<'_, Arc<Mutex<TimerState>>>,
    app: AppHandle,
) -> Result<WorkSession, String> {
    // Stop timer in worklog library
    let session = timesheet
        .stop_timer(None)
        .await
        .ok_or_else(|| "No active timer".to_string())?;

    // Update UI timer state
    timer_state.lock().unwrap().stop();

    // Update tray icon
    update_tray_menu(&app, false);

    // Emit event to refresh views
    let _ = app.emit("session-updated", ());

    Ok(session)
}

#[tauri::command]
async fn handle_sleep_stop(
    timesheet: tauri::State<'_, Arc<TimesheetIntegration>>,
    timer_state: tauri::State<'_, Arc<Mutex<TimerState>>>,
    app: AppHandle,
) -> Result<Option<WorkSession>, String> {
    // Stop the timer - user did not work during sleep
    match timesheet
        .stop_timer(Some("Stopped due to system sleep".to_string()))
        .await
    {
        Some(session) => {
            timer_state.lock().unwrap().stop();
            update_tray_menu(&app, false);
            let _ = app.emit("session-updated", ());
            Ok(Some(session))
        }
        None => Ok(None),
    }
}

#[tauri::command]
async fn handle_sleep_continue(app: AppHandle) -> Result<(), String> {
    // Continue the timer - user was working
    // No action needed, timer continues running
    let _ = app.emit("sleep-handled", "continued");
    Ok(())
}

#[tauri::command]
async fn test_sleep_notification(app: AppHandle) -> Result<(), String> {
    use tauri_plugin_notification::NotificationExt;

    eprintln!("🧪 Testing sleep notification...");

    // Show notification
    app.notification()
        .builder()
        .title("💤 System Wake - Timer Active")
        .body("Your computer was asleep for 15m.\n\nCheck the menubar to continue or stop your timer.")
        .sound("default")
        .show()
        .map_err(|e| format!("Failed to show notification: {e}"))?;

    eprintln!("✅ Test notification sent");

    // Simulate sleep 15 minutes ago
    let sleep_start = Utc::now() - chrono::Duration::minutes(15);

    // Emit the sleep-wake-detected event to update the menu
    let _ = app.emit(
        "sleep-wake-detected",
        serde_json::json!({
            "sleepDuration": "15m",
            "sleepStartTime": sleep_start.to_rfc3339(),
            "sleepDurationSeconds": 900
        }),
    );

    eprintln!("✅ Sleep-wake event emitted");

    Ok(())
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
fn get_current_time(timer_state: tauri::State<Arc<Mutex<TimerState>>>) -> String {
    timer_state.lock().unwrap().get_elapsed_time()
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
fn is_in_progress(timer_state: tauri::State<Arc<Mutex<TimerState>>>) -> bool {
    timer_state.lock().unwrap().is_running()
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
async fn get_all_sessions(
    timesheet: tauri::State<'_, Arc<TimesheetIntegration>>,
) -> Result<Vec<WorkSession>, String> {
    timesheet.get_all_timers().await.map_err(|e| e.to_string())
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
async fn get_session(
    timesheet: tauri::State<'_, Arc<TimesheetIntegration>>,
    id: String,
) -> Result<Option<WorkSession>, String> {
    // Get all timers and find the one with matching ID
    let all_sessions = timesheet
        .get_all_timers()
        .await
        .map_err(|e| e.to_string())?;

    Ok(all_sessions.into_iter().find(|s| s.id == id))
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
async fn add_session(
    timesheet: tauri::State<'_, Arc<TimesheetIntegration>>,
    start_date: String,
    end_date: Option<String>,
    issue_key: Option<String>,
    comment: Option<String>,
) -> Result<String, String> {
    use chrono::DateTime;

    // Parse dates
    let started_at: DateTime<chrono::Utc> = start_date
        .parse()
        .map_err(|e| format!("Invalid start date: {e}"))?;
    let stopped_at: Option<DateTime<chrono::Utc>> = end_date
        .map(|s| s.parse().map_err(|e| format!("Invalid end date: {e}")))
        .transpose()?;

    // Create worklog entry
    let mut entry = worklog_core::WorklogEntry::start_at(
        started_at.with_timezone(&chrono::Local),
        issue_key,
        comment,
    );

    // Set stopped_at if provided
    if let Some(stopped) = stopped_at {
        entry.stop(stopped.with_timezone(&chrono::Local));
    }

    // Add to database
    let id = timesheet
        .add_worklog_entry(&entry)
        .await
        .map_err(|e| e.to_string())?;

    Ok(id)
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
async fn update_session(
    timesheet: tauri::State<'_, Arc<TimesheetIntegration>>,
    id: String,
    start_date: String,
    end_date: Option<String>,
    issue_key: Option<String>,
    comment: Option<String>,
) -> Result<(), String> {
    use chrono::DateTime;

    // Parse dates
    let started_at: DateTime<chrono::Utc> = start_date
        .parse()
        .map_err(|e| format!("Invalid start date: {e}"))?;
    let stopped_at: Option<DateTime<chrono::Utc>> = end_date
        .map(|s| s.parse().map_err(|e| format!("Invalid end date: {e}")))
        .transpose()?;

    // Get existing entry
    let mut entry = timesheet
        .find_worklog_by_id(&id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Worklog entry {id} not found"))?;

    // Update fields
    entry.started_at = started_at.with_timezone(&chrono::Local);
    entry.stopped_at = stopped_at.map(|dt| dt.with_timezone(&chrono::Local));
    entry.issue_key = issue_key;
    entry.comment = comment;

    // Mark as modified locally (but keep provider_worklog_id for conflict detection)
    // Set updated_at to now, but DON'T update last_synced_at
    // This allows sync to detect conflicts if remote was also modified
    entry.synced_to_provider = false;
    entry.updated_at = chrono::Local::now();
    // Keep provider_worklog_id to maintain link to remote entry

    // Recalculate duration if stopped
    if let Some(stopped) = entry.stopped_at {
        let duration = stopped.signed_duration_since(entry.started_at);
        #[allow(clippy::cast_possible_truncation)]
        {
            entry.time_spent_seconds = Some(duration.num_seconds() as i32);
        }
    } else {
        entry.time_spent_seconds = None;
    }

    // Update in database
    timesheet
        .update_worklog_entry(&entry)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
async fn delete_session(
    timesheet: tauri::State<'_, Arc<TimesheetIntegration>>,
    id: String,
) -> Result<(), String> {
    timesheet
        .delete_worklog(&id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
async fn assign_issue_to_session(
    timesheet: tauri::State<'_, Arc<TimesheetIntegration>>,
    session_id: String,
    issue_key: String,
) -> Result<(), String> {
    timesheet
        .assign_issue_to_session(&session_id, &issue_key)
        .await
        .map_err(|e| e.to_string())
}

#[derive(serde::Serialize, serde::Deserialize)]
struct JiraIssue {
    key: String,
    summary: String,
    from_cache: bool,
}

#[tauri::command]
async fn get_all_jira_issues(
    timesheet: tauri::State<'_, Arc<TimesheetIntegration>>,
) -> Result<Vec<JiraIssue>, String> {
    if timesheet.is_local_only() {
        return Err(
            "Jira not configured. Please configure Jira in Settings to see issues.".to_string(),
        );
    }

    let issues = timesheet
        .get_all_issues()
        .await
        .map_err(|e| e.to_string())?;

    Ok(issues
        .into_iter()
        .map(|i| JiraIssue {
            key: i.key,
            summary: i.summary,
            from_cache: i.from_cache,
        })
        .collect())
}

#[tauri::command]
async fn sync_to_jira(
    timesheet: tauri::State<'_, Arc<TimesheetIntegration>>,
) -> Result<usize, String> {
    timesheet.sync_to_jira().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn sync_from_jira(
    timesheet: tauri::State<'_, Arc<TimesheetIntegration>>,
    days_back: Option<i64>,
) -> Result<usize, String> {
    timesheet
        .sync_from_jira(days_back)
        .await
        .map_err(|e| e.to_string())
}

#[derive(serde::Serialize, serde::Deserialize)]
struct JiraConfig {
    url: String,
    user: String,
    auth_type: String, // "token" or "oauth"
    token: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct OAuthConfig {
    client_id: String,
    client_secret: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct OAuthTokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: u64,
}

#[tauri::command]
#[allow(clippy::unnecessary_wraps)]
fn get_jira_config() -> Result<Option<JiraConfig>, String> {
    use worklog_config::config;

    match config::load_with_keychain_lookup() {
        Ok(cfg) => {
            let url = cfg
                .issue_tracker
                .url
                .trim_start_matches("jira://")
                .to_string();
            let url = if url.starts_with("http") {
                url
            } else {
                format!("https://{url}")
            };

            Ok(Some(JiraConfig {
                url,
                user: cfg.issue_tracker.get_username().unwrap_or("").to_string(),
                // Don't send the actual token back for security
                token: if cfg.issue_tracker.get_token().unwrap_or("").is_empty() {
                    String::new()
                } else {
                    "***".to_string()
                },
                auth_type: "token".to_string(), // Currently only token auth is used
            }))
        }
        Err(_) => Ok(None),
    }
}

#[tauri::command]
async fn save_jira_config(config: JiraConfig) -> Result<(), String> {
    use worklog_config::config::{self, AppConfiguration, IssueTrackerConfiguration};

    // Load existing config or create default
    let storage = config::load_with_keychain_lookup()
        .map(|cfg| cfg.storage)
        .unwrap_or_default();

    let url = config
        .url
        .trim_start_matches("https://")
        .trim_start_matches("http://");

    let mut tracker_config = std::collections::HashMap::new();
    tracker_config.insert("username".to_string(), config.user);
    tracker_config.insert("token".to_string(), config.token);

    let app_config = AppConfiguration {
        storage,
        issue_tracker: IssueTrackerConfiguration {
            url: format!("jira://{url}"),
            config: tracker_config,
        },
        jira: None,
        application_data: None,
    };

    config::save(&app_config).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
async fn test_jira_connection(config: JiraConfig) -> Result<String, String> {
    use std::collections::HashMap;
    use worklog_core::IssueTrackerClient;
    use worklog_jira_adapter::JiraAdapter;

    // Create temporary tracker client to test connection
    let url = config
        .url
        .trim_start_matches("https://")
        .trim_start_matches("http://");

    let mut tracker_config = HashMap::new();
    tracker_config.insert("username".to_string(), config.user);
    tracker_config.insert("token".to_string(), config.token);

    let tracker = JiraAdapter::from_url(&format!("jira://{url}"), &tracker_config)
        .map_err(|e| e.to_string())?;

    // Test connection by getting current user
    let user = tracker
        .get_current_user()
        .await
        .map_err(|e| e.to_string())?;

    Ok(format!("Connected successfully as: {}", user.display_name))
}

#[tauri::command]
fn remove_jira_config() -> Result<(), String> {
    use worklog_config::config;
    config::remove().map_err(|e| e.to_string())
}

#[tauri::command]
async fn start_oauth_flow(oauth_config: OAuthConfig) -> Result<String, String> {
    use std::sync::{Arc, Mutex};
    use tiny_http::{Response, Server};
    use url::Url;

    // Generate random state for CSRF protection
    let state = uuid::Uuid::new_v4().to_string();
    let state_clone = state.clone();

    // Start local server on port 8080 to receive callback
    let server = Server::http("127.0.0.1:8080")
        .map_err(|e| format!("Failed to start callback server: {e}"))?;
    let auth_code = Arc::new(Mutex::new(None::<String>));
    let auth_code_clone = auth_code.clone();

    // Build authorization URL
    let mut auth_url = Url::parse("https://auth.atlassian.com/authorize").unwrap();
    auth_url
        .query_pairs_mut()
        .append_pair("audience", "api.atlassian.com")
        .append_pair("client_id", &oauth_config.client_id)
        .append_pair(
            "scope",
            "read:jira-work read:jira-user write:jira-work offline_access",
        )
        .append_pair("redirect_uri", "http://127.0.0.1:8080/callback")
        .append_pair("state", &state)
        .append_pair("response_type", "code")
        .append_pair("prompt", "consent");

    let auth_url_string = auth_url.to_string();

    // Open browser to authorization URL
    if let Err(e) = open::that(&auth_url_string) {
        return Err(format!("Failed to open browser: {e}"));
    }

    // Wait for callback in a background task
    tokio::spawn(async move {
        if let Some(request) = server.incoming_requests().next() {
            let url = format!("http://127.0.0.1:8080{}", request.url());
            if let Ok(parsed_url) = Url::parse(&url) {
                let params: std::collections::HashMap<_, _> = parsed_url.query_pairs().collect();

                // Verify state to prevent CSRF
                if params.get("state") == Some(&state_clone.as_str().into()) {
                    if let Some(code) = params.get("code") {
                        *auth_code_clone.lock().unwrap() = Some(code.to_string());
                        let _ = request.respond(Response::from_string(
                            "Authorization successful! You can close this window and return to the app."
                        ));
                        return;
                    }
                }

                let _ = request.respond(Response::from_string(
                    "Authorization failed. Please try again.",
                ));
            }
        }
    });

    // Wait for authorization code (with timeout)
    for _ in 0..60 {
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        if let Some(code) = auth_code.lock().unwrap().as_ref() {
            return Ok(code.clone());
        }
    }

    Err("Authorization timeout. Please try again.".to_string())
}

#[tauri::command]
async fn exchange_oauth_code(
    code: String,
    oauth_config: OAuthConfig,
) -> Result<OAuthTokenResponse, String> {
    let client = reqwest::Client::new();

    let params = [
        ("grant_type", "authorization_code"),
        ("client_id", &oauth_config.client_id),
        ("client_secret", &oauth_config.client_secret),
        ("code", &code),
        ("redirect_uri", "http://127.0.0.1:8080/callback"),
    ];

    let response = client
        .post("https://auth.atlassian.com/oauth/token")
        .form(&params)
        .send()
        .await
        .map_err(|e| format!("Failed to exchange code: {e}"))?;

    if !response.status().is_success() {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        return Err(format!("Token exchange failed: {error_text}"));
    }

    response
        .json::<OAuthTokenResponse>()
        .await
        .map_err(|e| format!("Failed to parse token response: {e}"))
}

#[derive(serde::Deserialize)]
struct Resource {
    url: String,
}

#[derive(serde::Deserialize)]
struct JiraUser {
    #[serde(rename = "displayName")]
    display_name: String,
    #[serde(rename = "emailAddress")]
    email_address: String,
}

#[tauri::command]
async fn get_oauth_user_info(access_token: String) -> Result<String, String> {
    let client = reqwest::Client::new();

    // First, get accessible resources (Jira sites)
    let resources_response = client
        .get("https://api.atlassian.com/oauth/token/accessible-resources")
        .bearer_auth(&access_token)
        .send()
        .await
        .map_err(|e| format!("Failed to get resources: {e}"))?;

    let resources: Vec<Resource> = resources_response
        .json()
        .await
        .map_err(|e| format!("Failed to parse resources: {e}"))?;

    if resources.is_empty() {
        return Err("No Jira sites found for this account".to_string());
    }

    // Use the first site
    let site = &resources[0];

    // Get user info from Jira
    let user_response = client
        .get(format!("{}/rest/api/3/myself", site.url))
        .bearer_auth(&access_token)
        .send()
        .await
        .map_err(|e| format!("Failed to get user info: {e}"))?;

    let user: JiraUser = user_response
        .json()
        .await
        .map_err(|e| format!("Failed to parse user: {e}"))?;

    Ok(serde_json::json!({
        "email": user.email_address,
        "displayName": user.display_name,
        "jiraUrl": site.url,
    })
    .to_string())
}

fn update_tray_menu(app: &AppHandle, _in_progress: bool) {
    // Recreate the tray menu
    if let Ok(menu) = create_tray_menu(app) {
        if let Some(tray) = app.tray_by_id("main-tray") {
            let _ = tray.set_menu(Some(menu));
        }
    }
}

fn create_tray_menu(app: &AppHandle) -> Result<Menu<tauri::Wry>, Box<dyn std::error::Error>> {
    let mut menu_items: Vec<Box<dyn tauri::menu::IsMenuItem<tauri::Wry>>> = vec![];

    // Check if there's an active timer using the TimerState (no async needed)
    let timer_state = app.try_state::<Arc<Mutex<TimerState>>>();
    let has_active_timer = timer_state
        .as_ref()
        .is_some_and(|state| state.lock().unwrap().is_running());

    // Check if we're awaiting a sleep wake response
    let sleep_wake_state = app.try_state::<Arc<Mutex<SleepWakeState>>>();
    let awaiting_response = sleep_wake_state
        .as_ref()
        .is_some_and(|state| state.lock().unwrap().awaiting_response);

    if awaiting_response {
        // Show sleep wake response options
        let sleep_duration = sleep_wake_state
            .as_ref()
            .map(|state| state.lock().unwrap().sleep_duration.clone())
            .unwrap_or_default();

        let title = MenuItem::with_id(
            app,
            "sleep_title",
            format!("💤 Computer slept for {sleep_duration}"),
            false,
            None::<&str>,
        )?;
        menu_items.push(Box::new(title));

        let subtitle = MenuItem::with_id(
            app,
            "sleep_subtitle",
            "What were you doing?",
            false,
            None::<&str>,
        )?;
        menu_items.push(Box::new(subtitle));
        menu_items.push(Box::new(PredefinedMenuItem::separator(app)?));

        // Option 1: Continue (include sleep time)
        let continue_timer = MenuItem::with_id(
            app,
            "sleep_continue",
            "▸ Keep Working (count sleep time)",
            true,
            None::<&str>,
        )?;
        menu_items.push(Box::new(continue_timer));

        // Option 2: Stop at sleep time (break)
        let stop_at_sleep = MenuItem::with_id(
            app,
            "sleep_stop_at_sleep",
            "▸ I Took a Break (stop at sleep)",
            true,
            None::<&str>,
        )?;
        menu_items.push(Box::new(stop_at_sleep));

        // Option 3: Worked on something else (backfill to different code)
        let worked_other = MenuItem::with_id(
            app,
            "sleep_worked_other",
            "▸ I Worked on Something Else...",
            true,
            None::<&str>,
        )?;
        menu_items.push(Box::new(worked_other));

        // Option 4: Stop and start fresh
        let stop_start_new = MenuItem::with_id(
            app,
            "sleep_stop_start_new",
            "▸ Stop & Start New Timer...",
            true,
            None::<&str>,
        )?;
        menu_items.push(Box::new(stop_start_new));

        menu_items.push(Box::new(PredefinedMenuItem::separator(app)?));
    } else {
        // Show normal menu
        let start_submenu = create_start_submenu(app, !has_active_timer)?;
        let stop = MenuItem::with_id(app, "stop", "Stop", has_active_timer, None::<&str>)?;

        menu_items.push(Box::new(start_submenu));
        menu_items.push(Box::new(stop));
    }

    // Common items
    // Disable sync if in local-only mode (no issue tracker)
    let sync_enabled = app
        .try_state::<Arc<TimesheetIntegration>>()
        .is_some_and(|integration| !integration.is_local_only());

    let refresh = MenuItem::with_id(app, "refresh", "Sync", sync_enabled, None::<&str>)?;
    let week_view = MenuItem::with_id(app, "week_view", "Week View", true, None::<&str>)?;
    let sessions = MenuItem::with_id(app, "sessions", "Sessions", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

    menu_items.push(Box::new(refresh));
    menu_items.push(Box::new(PredefinedMenuItem::separator(app)?));
    menu_items.push(Box::new(week_view));
    menu_items.push(Box::new(sessions));
    menu_items.push(Box::new(PredefinedMenuItem::separator(app)?));
    menu_items.push(Box::new(settings));
    menu_items.push(Box::new(quit));

    let items_refs: Vec<&dyn tauri::menu::IsMenuItem<tauri::Wry>> =
        menu_items.iter().map(std::convert::AsRef::as_ref).collect();

    Ok(Menu::with_items(app, &items_refs)?)
}

fn create_start_submenu(
    app: &AppHandle,
    enabled: bool,
) -> Result<tauri::menu::Submenu<tauri::Wry>, Box<dyn std::error::Error>> {
    use tauri::menu::Submenu;

    let timesheet = app.state::<Arc<TimesheetIntegration>>();

    // Get recent issues from the database
    let recent_issues =
        match tauri::async_runtime::block_on(async { timesheet.get_all_issues().await }) {
            Ok(issues) => {
                // Get unique issues by key, limit to most recent 10
                let mut seen_keys = std::collections::HashSet::new();
                let mut unique_issues = Vec::new();

                for issue in issues {
                    if seen_keys.insert(issue.key.clone()) {
                        unique_issues.push((issue.key, issue.summary));
                        if unique_issues.len() >= 10 {
                            break;
                        }
                    }
                }

                unique_issues.sort_by(|a, b| a.0.cmp(&b.0));
                unique_issues
            }
            Err(e) => {
                eprintln!("Failed to load recent issues: {e}");
                vec![]
            }
        };

    // Create menu items - start with "No Issue" option
    let mut menu_items: Vec<Box<dyn tauri::menu::IsMenuItem<tauri::Wry>>> = vec![];

    // Add "No Issue" as first option
    let no_issue = MenuItem::with_id(app, "start", "(No Issue)", enabled, None::<&str>)?;
    menu_items.push(Box::new(no_issue));

    // Add separator if there are issues
    if !recent_issues.is_empty() {
        menu_items.push(Box::new(PredefinedMenuItem::separator(app)?));

        // Add recent issues with summaries
        for (issue_key, summary) in recent_issues {
            let id = format!("start_with:{issue_key}");

            // Truncate summary if too long for menu display
            let truncated_summary = if summary.len() > 50 {
                format!("{}...", &summary[..47])
            } else {
                summary
            };

            let label = format!("{issue_key} – {truncated_summary}");
            let item = MenuItem::with_id(app, id, &label, enabled, None::<&str>)?;
            menu_items.push(Box::new(item));
        }
    }

    let items_refs: Vec<&dyn tauri::menu::IsMenuItem<tauri::Wry>> =
        menu_items.iter().map(std::convert::AsRef::as_ref).collect();

    Ok(Submenu::with_items(app, "Start", enabled, &items_refs)?)
}

fn handle_tray_event(_tray: &TrayIcon, _event: TrayIconEvent) {
    // Handle tray icon events if needed
}

#[allow(clippy::needless_pass_by_value)]
#[allow(clippy::too_many_lines)] // Menu has many event types to handle
fn handle_menu_event(app: &AppHandle, event: tauri::menu::MenuEvent) {
    let event_id = event.id().as_ref();

    // Handle "start_with:ISSUE-KEY" menu items
    if let Some(issue_key) = event_id.strip_prefix("start_with:") {
        let timesheet = app.state::<Arc<TimesheetIntegration>>();
        let timer_state = app.state::<Arc<Mutex<TimerState>>>();

        let issue_key_owned = issue_key.to_string();

        if let Err(e) = tauri::async_runtime::block_on(async {
            timesheet.start_timer(Some(&issue_key_owned), None).await
        }) {
            eprintln!("Failed to start timer for {issue_key_owned}: {e}");
        } else {
            println!("Started timer for issue: {issue_key_owned}");
            timer_state.lock().unwrap().start();
            update_tray_menu(app, true);
            let _ = app.emit("timer-started", ());
        }
        return;
    }

    match event_id {
        "sleep_continue" => {
            // User was working during sleep - keep timer running with sleep time counted
            let sleep_wake_state = app.state::<Arc<Mutex<SleepWakeState>>>();
            sleep_wake_state.lock().unwrap().clear();
            update_tray_menu(app, true);
            eprintln!("✓ User chose to continue timer (count sleep time)");
        }
        "sleep_stop_at_sleep" => {
            // User took a break - stop timer at the moment computer went to sleep
            let timesheet = app.state::<Arc<TimesheetIntegration>>();
            let timer_state = app.state::<Arc<Mutex<TimerState>>>();
            let sleep_wake_state = app.state::<Arc<Mutex<SleepWakeState>>>();

            // Get the sleep start time to stop timer at that exact moment
            let stop_time = sleep_wake_state.lock().unwrap().sleep_start_time;

            if let Some(_session) = tauri::async_runtime::block_on(async {
                timesheet.stop_timer_at(stop_time, None).await
            }) {
                timer_state.lock().unwrap().stop();
                sleep_wake_state.lock().unwrap().clear();
                update_tray_menu(app, false);
                let _ = app.emit("session-updated", ());
                eprintln!("✓ User chose to stop timer at sleep time (break)");
            } else {
                eprintln!("No active timer to stop");
            }
        }
        "sleep_worked_other" => {
            // User worked on something else during sleep
            // TODO: Show dialog/window to select which issue to backfill the sleep time to
            eprintln!("🔄 User wants to backfill sleep time to different issue");
            let sleep_wake_state = app.state::<Arc<Mutex<SleepWakeState>>>();
            sleep_wake_state.lock().unwrap().clear();
            update_tray_menu(app, false);

            // For now, just clear the state and let user manually handle it
            eprintln!("⚠️ Backfill to different issue not yet implemented - clearing state");
        }
        "sleep_stop_start_new" => {
            // Stop current timer and start a new one (no backfill)
            let timesheet = app.state::<Arc<TimesheetIntegration>>();
            let timer_state = app.state::<Arc<Mutex<TimerState>>>();
            let sleep_wake_state = app.state::<Arc<Mutex<SleepWakeState>>>();

            // Get the sleep start time to stop timer at that exact moment
            let stop_time = sleep_wake_state.lock().unwrap().sleep_start_time;

            // Stop the current timer at sleep time
            if let Some(_session) = tauri::async_runtime::block_on(async {
                timesheet.stop_timer_at(stop_time, None).await
            }) {
                timer_state.lock().unwrap().stop();
                eprintln!("✓ Stopped timer at sleep time");
            }

            sleep_wake_state.lock().unwrap().clear();
            update_tray_menu(app, false);

            // TODO: Show dialog/window to select which issue to start fresh
            // For now, user can use the Start menu
            eprintln!("🆕 User wants to start new timer - use Start menu");
        }
        "start" => {
            let timesheet = app.state::<Arc<TimesheetIntegration>>();
            let timer_state = app.state::<Arc<Mutex<TimerState>>>();

            // Start timer without issue key from menu
            if let Err(e) =
                tauri::async_runtime::block_on(async { timesheet.start_timer(None, None).await })
            {
                eprintln!("Failed to start timer: {e}");
            } else {
                timer_state.lock().unwrap().start();
                update_tray_menu(app, true);
                let _ = app.emit("timer-started", ());
            }
        }
        "stop" => {
            let timesheet = app.state::<Arc<TimesheetIntegration>>();
            let timer_state = app.state::<Arc<Mutex<TimerState>>>();

            if let Some(_session) =
                tauri::async_runtime::block_on(async { timesheet.stop_timer(None).await })
            {
                timer_state.lock().unwrap().stop();
                update_tray_menu(app, false);
                let _ = app.emit("session-updated", ());
            } else {
                eprintln!("No active timer to stop");
            }
        }
        "refresh" => {
            let timesheet = app.state::<Arc<TimesheetIntegration>>();

            // Run full sync: fetch, compare, auto-merge
            eprintln!("🔄 Starting synchronization...");
            match tauri::async_runtime::block_on(async { timesheet.sync(Some(30)).await }) {
                Ok(result) => {
                    if result.added > 0 || result.updated > 0 || result.uploaded > 0 {
                        eprintln!(
                            "✓ Sync complete: {} added, {} updated, {} uploaded",
                            result.added, result.updated, result.uploaded
                        );
                    } else {
                        eprintln!("✓ Everything is in sync");
                    }

                    if result.conflicts > 0 {
                        eprintln!(
                            "⚠️  {} conflicts detected (manual resolution required)",
                            result.conflicts
                        );
                    }

                    if result.deleted_local > 0 || result.deleted_remote > 0 {
                        eprintln!(
                            "🗑️  Deletions: {} local, {} remote",
                            result.deleted_local, result.deleted_remote
                        );
                    }
                }
                Err(e) => {
                    eprintln!("⚠️  Sync failed: {e}");
                }
            }

            // Emit event to all windows to refresh their UI
            if let Err(e) = app.emit("refresh-data", ()) {
                eprintln!("Failed to emit refresh event: {e}");
            }
        }
        "week_view" => {
            if let Some(window) = app.get_webview_window("week_view") {
                let _ = window.show();
                let _ = window.set_focus();
            } else {
                #[cfg(target_os = "macos")]
                {
                    use tauri::TitleBarStyle;
                    let window = WebviewWindowBuilder::new(
                        app,
                        "week_view",
                        WebviewUrl::App("src/index.html#/week".into()),
                    )
                    .title("Week View")
                    .inner_size(1200.0, 900.0)
                    .title_bar_style(TitleBarStyle::Overlay)
                    .traffic_light_position(LogicalPosition::new(19.0, 29.0))
                    .build();

                    if let Ok(win) = window {
                        win.set_transparent_titlebar(true);
                    }
                }

                #[cfg(not(target_os = "macos"))]
                {
                    let _ = WebviewWindowBuilder::new(
                        app,
                        "week_view",
                        WebviewUrl::App("src/index.html#/week".into()),
                    )
                    .title("Week View")
                    .inner_size(1200.0, 900.0)
                    .build();
                }
            }
        }
        "sessions" => {
            if let Some(window) = app.get_webview_window("sessions") {
                let _ = window.show();
                let _ = window.set_focus();
            } else {
                #[cfg(target_os = "macos")]
                {
                    use tauri::TitleBarStyle;
                    let window = WebviewWindowBuilder::new(
                        app,
                        "sessions",
                        WebviewUrl::App("src/index.html#/sessions".into()),
                    )
                    .title("Sessions")
                    .inner_size(1000.0, 700.0)
                    .title_bar_style(TitleBarStyle::Overlay)
                    .traffic_light_position(LogicalPosition::new(19.0, 29.0))
                    .build();

                    if let Ok(win) = window {
                        win.set_transparent_titlebar(true);
                    }
                }

                #[cfg(not(target_os = "macos"))]
                {
                    let _ = WebviewWindowBuilder::new(
                        app,
                        "sessions",
                        WebviewUrl::App("src/index.html#/sessions".into()),
                    )
                    .title("Sessions")
                    .inner_size(1000.0, 700.0)
                    .build();
                }
            }
        }
        "settings" => {
            if let Some(window) = app.get_webview_window("settings") {
                let _ = window.show();
                let _ = window.set_focus();
            } else {
                let _ = WebviewWindowBuilder::new(
                    app,
                    "settings",
                    WebviewUrl::App("src/index.html#/settings".into()),
                )
                .title("Settings")
                .inner_size(700.0, 800.0)
                .build();
            }
        }
        "quit" => {
            let timesheet = app.state::<Arc<TimesheetIntegration>>();
            let timer_state = app.state::<Arc<Mutex<TimerState>>>();

            if timer_state.lock().unwrap().is_running() {
                let _ = tauri::async_runtime::block_on(async { timesheet.stop_timer(None).await });
            }
            app.exit(0);
        }
        _ => {}
    }
}

#[allow(clippy::too_many_lines)] // Main function contains app setup
fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            // Initialize timesheet integration - try with Jira first, fall back to local-only
            let timesheet_integration = match TimesheetIntegration::new() {
                Ok(integration) => {
                    eprintln!("✓ Timesheet integration initialized with Jira support");
                    Arc::new(integration)
                }
                Err(e) => {
                    eprintln!("⚠ Jira integration unavailable: {e}");
                    eprintln!("  Falling back to local-only mode (no Jira sync)");
                    eprintln!("  To enable Jira sync:");
                    eprintln!("  1. Create config: ~/Library/Preferences/com.norn.timesheet");
                    eprintln!("  2. Or run: cd ~/timesheet && cargo run --bin timesheet init");

                    // Fall back to local-only mode
                    Arc::new(
                        TimesheetIntegration::new_local_only()
                            .expect("Failed to initialize in local-only mode"),
                    )
                }
            };

            // Check if there's an active timer and restore the UI state
            let timer_state = Arc::new(Mutex::new(TimerState::new()));
            if let Ok(Some(active_timer)) = tauri::async_runtime::block_on(async {
                timesheet_integration.get_active_timer().await
            }) {
                eprintln!("✓ Restoring active timer from {}", active_timer.start_date);
                timer_state
                    .lock()
                    .unwrap()
                    .restore_from(active_timer.start_date);
            }

            // Initialize system monitors
            let monitors = SystemMonitors::new();
            monitors.start(app.handle());

            // Initialize sleep wake state
            let sleep_wake_state = Arc::new(Mutex::new(SleepWakeState::new()));

            // Set up state management
            app.manage(timesheet_integration);
            app.manage(timer_state.clone());
            app.manage(sleep_wake_state.clone());

            // Listen for sleep-wake-detected events
            let app_for_sleep = app.handle().clone();
            let sleep_state_clone = sleep_wake_state.clone();
            app.listen("sleep-wake-detected", move |event| {
                eprintln!("💤 Sleep-wake detected event received");

                // Parse the JSON payload
                if let Ok(payload) = serde_json::from_str::<serde_json::Value>(event.payload()) {
                    let duration = payload
                        .get("sleepDuration")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown");
                    let sleep_start_str = payload
                        .get("sleepStartTime")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    let duration_secs = payload
                        .get("sleepDurationSeconds")
                        .and_then(serde_json::Value::as_i64)
                        .unwrap_or(0);

                    if let Ok(sleep_start) = DateTime::parse_from_rfc3339(sleep_start_str) {
                        eprintln!(
                            "💤 Setting sleep wake state: duration={duration}, start={sleep_start}"
                        );
                        sleep_state_clone.lock().unwrap().set_awaiting_response(
                            duration.to_string(),
                            sleep_start.with_timezone(&Utc),
                            duration_secs,
                        );

                        // Update menu on a separate thread to avoid runtime conflicts
                        let app_clone = app_for_sleep.clone();
                        thread::spawn(move || {
                            update_tray_menu(&app_clone, true);
                        });
                    } else {
                        eprintln!("❌ Failed to parse sleep start time");
                    }
                } else {
                    eprintln!("❌ Failed to parse sleep-wake-detected payload");
                }
            });

            // Listen for notification click events
            let app_for_notification = app.handle().clone();
            app.listen("notification", move |event| {
                eprintln!("🔔 Notification event received: {:?}", event.payload());

                // When notification is clicked, open/focus a window and show the dialog
                if let Some(window) = app_for_notification.get_webview_window("sessions") {
                    let _ = window.set_focus();
                    let _ = window.unminimize();
                    eprintln!("✅ Focused Sessions window");
                } else if let Some(window) = app_for_notification.get_webview_window("week_view") {
                    let _ = window.set_focus();
                    let _ = window.unminimize();
                    eprintln!("✅ Focused Week view window");
                } else {
                    // No window open, create Sessions window
                    eprintln!("📱 Creating Sessions window...");
                    let _ = WebviewWindowBuilder::new(
                        &app_for_notification,
                        "sessions",
                        WebviewUrl::App("index.html#/sessions".into()),
                    )
                    .title("Timesheet - Sessions")
                    .inner_size(900.0, 700.0)
                    .build();
                }
            });

            // Create tray menu
            let tray_menu = create_tray_menu(app.handle())?;

            // Load the SF Symbol icon exported from macOS
            let icon_bytes = include_bytes!("../icons/tray_icon.png");
            let img = image::load_from_memory(icon_bytes)?;
            let rgba = img.to_rgba8();
            let (width, height) = rgba.dimensions();
            let icon = Image::new_owned(rgba.into_raw(), width, height);

            // Create tray icon with template mode for macOS (auto light/dark mode)
            let _tray_icon = TrayIconBuilder::with_id("main-tray")
                .icon(icon)
                .icon_as_template(true) // Mark as template for automatic color inversion
                .title("00:00:00") // Initial time display
                .tooltip("Timesheet")
                .menu(&tray_menu)
                .show_menu_on_left_click(true)
                .on_menu_event(handle_menu_event)
                .on_tray_icon_event(handle_tray_event)
                .build(app)?;

            // Start timer update loop
            let app_handle = app.handle().clone();
            let timer_state_clone = timer_state.clone();
            std::thread::spawn(move || loop {
                std::thread::sleep(std::time::Duration::from_secs(1));
                let state = timer_state_clone.lock().unwrap();
                let time = state.get_elapsed_time();
                let in_progress = state.is_running();
                drop(state);

                // Update the tray icon title with current time
                if let Some(tray) = app_handle.tray_by_id("main-tray") {
                    let _ = tray.set_title(Some(&time));
                }

                let _ = app_handle.emit("timer-update", TimerUpdatePayload { time, in_progress });
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            start_timer,
            stop_timer,
            get_active_timer,
            get_current_time,
            is_in_progress,
            handle_sleep_stop,
            handle_sleep_continue,
            test_sleep_notification,
            get_all_sessions,
            get_session,
            add_session,
            update_session,
            delete_session,
            assign_issue_to_session,
            get_all_jira_issues,
            sync_to_jira,
            sync_from_jira,
            get_jira_config,
            save_jira_config,
            test_jira_connection,
            remove_jira_config,
            start_oauth_flow,
            exchange_oauth_code,
            get_oauth_user_info,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
