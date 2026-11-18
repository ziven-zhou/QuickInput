mod confirmation;
mod platform;

use std::path::PathBuf;
use std::str::FromStr;
use std::sync::{atomic::AtomicBool, Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use confirmation::{ConfirmAction, ConfirmationLease};
use serde::{Deserialize, Serialize};
use tauri::menu::{MenuBuilder, MenuItemBuilder};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, LogicalSize, Manager, Position, State, WindowEvent};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

const MIN_INPUT_FONT_SIZE: u32 = 12;
const MAX_INPUT_FONT_SIZE: u32 = 32;
const DEFAULT_INPUT_FONT_SIZE: u32 = 14;
const DEFAULT_WINDOW_WIDTH: u32 = 720;
const DEFAULT_WINDOW_HEIGHT: u32 = 240;
const MIN_WINDOW_WIDTH: u32 = 400;
const MAX_WINDOW_WIDTH: u32 = 1200;
const MIN_WINDOW_HEIGHT: u32 = 240;
const MAX_WINDOW_HEIGHT: u32 = 800;
const LEGACY_DEFAULT_WINDOW_WIDTH: u32 = 600;
const LEGACY_DEFAULT_WINDOW_HEIGHT: u32 = 400;
const MIN_BG_OPACITY: f64 = 0.1;
const MAX_BG_OPACITY: f64 = 1.0;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Settings {
    pub trigger_hotkey: String,
    #[serde(default = "default_content_hotkey", alias = "history_hotkey")]
    pub content_hotkey: String,
    #[serde(default = "default_settings_hotkey")]
    pub settings_hotkey: String,
    pub confirm_hotkey: String,
    #[serde(default = "default_cancel_hotkey")]
    pub cancel_hotkey: String,
    pub window_position: String,
    #[serde(default = "default_window_width")]
    pub window_width: u32,
    #[serde(default = "default_window_height")]
    pub window_height: u32,
    #[serde(default = "default_bg_opacity")]
    pub bg_opacity: f64,
    pub default_mode: String,
    #[serde(default)]
    pub pure_mode: bool,
    #[serde(default = "default_input_font_size")]
    pub input_font_size: u32,
    #[serde(default)]
    pub remember_window_position: bool,
    #[serde(default)]
    pub window_x: Option<i32>,
    #[serde(default)]
    pub window_y: Option<i32>,
    #[serde(default = "default_true")]
    pub restore_clipboard: bool,
    pub autostart: bool,
    #[serde(default)]
    pub keep_open_on_confirm: bool,
    #[serde(default)]
    pub paste_and_send_enabled: bool,
}

fn default_content_hotkey() -> String {
    "Alt+R".into()
}

fn default_settings_hotkey() -> String {
    "Alt+S".into()
}

fn default_cancel_hotkey() -> String {
    "Escape".into()
}

fn default_true() -> bool {
    true
}

fn default_window_width() -> u32 {
    DEFAULT_WINDOW_WIDTH
}

fn default_window_height() -> u32 {
    DEFAULT_WINDOW_HEIGHT
}

fn default_bg_opacity() -> f64 {
    0.97
}

fn default_input_font_size() -> u32 {
    DEFAULT_INPUT_FONT_SIZE
}

fn normalize_input_font_size(value: u32) -> u32 {
    if (MIN_INPUT_FONT_SIZE..=MAX_INPUT_FONT_SIZE).contains(&value) {
        value
    } else {
        DEFAULT_INPUT_FONT_SIZE
    }
}

fn normalize_content_mode(mode: &str) -> String {
    match mode.trim().to_ascii_lowercase().as_str() {
        "markdown" => "markdown".into(),
        _ => "plain".into(),
    }
}

fn normalize_window_width(width: u32) -> u32 {
    width.clamp(MIN_WINDOW_WIDTH, MAX_WINDOW_WIDTH)
}

fn normalize_window_height(height: u32) -> u32 {
    height.clamp(MIN_WINDOW_HEIGHT, MAX_WINDOW_HEIGHT)
}

fn normalize_bg_opacity(opacity: f64) -> f64 {
    if opacity.is_finite() {
        opacity.clamp(MIN_BG_OPACITY, MAX_BG_OPACITY)
    } else {
        default_bg_opacity()
    }
}

fn normalize_window_position(position: &str) -> String {
    match position.trim().to_ascii_lowercase().as_str() {
        "cursor" => "cursor".into(),
        _ => "center".into(),
    }
}

fn normalize_settings(settings: &mut Settings) {
    settings.input_font_size = normalize_input_font_size(settings.input_font_size);
    settings.default_mode = normalize_content_mode(&settings.default_mode);
    if settings.window_width == LEGACY_DEFAULT_WINDOW_WIDTH
        && settings.window_height == LEGACY_DEFAULT_WINDOW_HEIGHT
    {
        settings.window_width = DEFAULT_WINDOW_WIDTH;
        settings.window_height = DEFAULT_WINDOW_HEIGHT;
    }
    settings.window_width = normalize_window_width(settings.window_width);
    settings.window_height = normalize_window_height(settings.window_height);
    settings.bg_opacity = normalize_bg_opacity(settings.bg_opacity);
    settings.window_position = normalize_window_position(&settings.window_position);
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            trigger_hotkey: "Alt+Space".into(),
            content_hotkey: default_content_hotkey(),
            settings_hotkey: default_settings_hotkey(),
            confirm_hotkey: "Alt+Enter".into(),
            cancel_hotkey: "Escape".into(),
            window_position: "center".into(),
            window_width: DEFAULT_WINDOW_WIDTH,
            window_height: DEFAULT_WINDOW_HEIGHT,
            bg_opacity: 0.97,
            default_mode: "plain".into(),
            pure_mode: false,
            input_font_size: DEFAULT_INPUT_FONT_SIZE,
            remember_window_position: false,
            window_x: None,
            window_y: None,
            restore_clipboard: true,
            autostart: false,
            keep_open_on_confirm: false,
            paste_and_send_enabled: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ContentEntry {
    pub id: String,
    #[serde(default)]
    pub title: Option<String>,
    pub text: String,
    pub mode: String,
    pub created_at: u64,
    pub updated_at: u64,
}

pub struct AppState {
    settings: Mutex<Settings>,
    previous_app_pid: Mutex<Option<i32>>,
    registered_hotkeys: Mutex<Vec<String>>,
    contents: Mutex<Vec<ContentEntry>>,
    paste_in_progress: Arc<AtomicBool>,
}

struct ClipboardSnapshot {
    text: Option<String>,
    html: Option<String>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            settings: Mutex::new(Settings::default()),
            previous_app_pid: Mutex::new(None),
            registered_hotkeys: Mutex::new(Vec::new()),
            contents: Mutex::new(Vec::new()),
            paste_in_progress: Arc::new(AtomicBool::new(false)),
        }
    }
}

#[tauri::command]
fn get_settings(state: State<'_, AppState>) -> Settings {
    state.settings.lock().unwrap().clone()
}

#[tauri::command]
fn save_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    mut settings: Settings,
) -> Result<(), String> {
    normalize_settings(&mut settings);
    validate_hotkey_conflicts(&settings)?;

    let path = settings_path(&app)?;
    let previous = state.settings.lock().unwrap().clone();
    if let Err(err) = apply_hotkeys(&app, &settings) {
        let _ = apply_hotkeys(&app, &previous);
        return Err(err);
    }

    if let Err(err) = apply_autostart(&app, settings.autostart) {
        let _ = apply_hotkeys(&app, &previous);
        return Err(err);
    }

    let mut current_settings = state.settings.lock().unwrap();
    settings.window_x = current_settings.window_x;
    settings.window_y = current_settings.window_y;
    let content = match serde_json::to_string_pretty(&settings) {
        Ok(content) => content,
        Err(err) => {
            drop(current_settings);
            let _ = apply_hotkeys(&app, &previous);
            let _ = apply_autostart(&app, previous.autostart);
            return Err(err.to_string());
        }
    };
    if let Err(err) = std::fs::write(path, content).map_err(|e| e.to_string()) {
        let _ = apply_hotkeys(&app, &previous);
        let _ = apply_autostart(&app, previous.autostart);
        return Err(err);
    }

    *current_settings = settings.clone();
    drop(current_settings);

    let _ = app.emit("page-shortcuts-updated", ());
    Ok(())
}

#[tauri::command]
fn confirm_input(
    app: AppHandle,
    state: State<'_, AppState>,
    text: String,
    mode: String,
    rendered_html: Option<String>,
    attempt_id: u64,
    action: Option<ConfirmAction>,
) -> Result<(), String> {
    if text.trim().is_empty() {
        return Err("内容为空，无法粘贴".into());
    }
    let settings = state.settings.lock().unwrap().clone();
    let action = action.unwrap_or_default();
    action.validate(settings.paste_and_send_enabled)?;
    let lease = ConfirmationLease::acquire(state.paste_in_progress.clone())?;

    if !platform::can_simulate_paste() {
        write_paste_diag("ax_trusted=false target_pid=0 front_after_activate=0 front_after_paste=0 result=permission_denied");
        return Err("QuickInput 没有辅助功能权限，请在系统设置中授权 /Applications/QuickInput.app 后重启应用".into());
    }

    let previous_clipboard = clipboard_snapshot();
    let mode = normalize_content_mode(&mode);
    clipboard_set_for_mode(&text, &mode, rendered_html).map_err(|e| format!("clipboard: {}", e))?;
    let previous_pid = state.previous_app_pid.lock().unwrap().take();
    let _ = persist_main_window_position(&app);
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.hide();
    }

    std::thread::spawn(move || {
        // Hold the lease until restoration and all synthetic input have finished.
        std::thread::sleep(Duration::from_millis(400));
        let target_pid = previous_pid.unwrap_or_else(platform::get_frontmost_pid);
        if target_pid > 0 && target_pid != std::process::id() as i32 {
            platform::activate_pid(target_pid);
        }
        std::thread::sleep(Duration::from_millis(150));
        let front_after_activate = platform::get_frontmost_pid();
        let result = confirmation::perform_confirmation(
            action,
            if target_pid == std::process::id() as i32 {
                0
            } else {
                target_pid
            },
            &mut platform::SystemInput,
        );
        let front_after_paste = platform::get_frontmost_pid();
        let result_label = match &result {
            Ok(_) => "ok".to_string(),
            Err(err) => format!(
                "error:{} pasted={}",
                err.message.replace('\n', " "),
                err.pasted
            ),
        };
        write_paste_diag(&format!(
            "ax_trusted=true target_pid={} front_after_activate={} front_after_paste={} action={:?} result={}",
            target_pid, front_after_activate, front_after_paste, action, result_label
        ));
        // On paste errors, allow any already posted events to settle before restoration.
        if result.as_ref().is_err_and(|error| !error.pasted) {
            std::thread::sleep(Duration::from_millis(480));
        }
        if settings.restore_clipboard {
            let _ = restore_clipboard_snapshot(previous_clipboard);
        }

        if let Err(error) = &result {
            if !error.pasted && target_pid > 0 && target_pid != std::process::id() as i32 {
                *app.state::<AppState>().previous_app_pid.lock().unwrap() = Some(target_pid);
            }
            let _ = app.emit_to(
                "main",
                "paste-error",
                serde_json::json!({
                    "attempt_id": attempt_id, "message": error.message, "pasted": error.pasted,
                }),
            );
        }
        // Release before notifying the frontend so its next attempt can acquire the lease.
        drop(lease);
        let _ = app.emit_to(
            "main",
            "paste-complete",
            serde_json::json!({ "attempt_id": attempt_id }),
        );
        if result.is_err() || settings.keep_open_on_confirm {
            if let Some(win) = app.get_webview_window("main") {
                let _ = win.show();
                let _ = win.unminimize();
                let _ = win.set_focus();
            }
        }
    });
    Ok(())
}

fn write_paste_diag(message: &str) {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let line = format!("{} {}\n", millis, message);
    let _ = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("/tmp/quickinput_paste_diag.log")
        .and_then(|mut file| {
            use std::io::Write;
            file.write_all(line.as_bytes())
        });
}

#[tauri::command]
fn cancel_input(
    app: AppHandle,
    state: State<'_, AppState>,
    text: String,
    mode: String,
) -> Result<(), String> {
    let _ = (text, mode);
    let _ = persist_main_window_position(&app);
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.hide();
    }
    let pid = state.previous_app_pid.lock().unwrap().take();
    if let Some(pid) = pid {
        platform::activate_pid(pid);
    }
    Ok(())
}

#[tauri::command]
fn open_settings(app: AppHandle) -> Result<(), String> {
    if let Some(win) = app.get_webview_window("settings") {
        let _ = win.center();
        let _ = win.show();
        let _ = win.set_focus();
    }
    Ok(())
}

#[tauri::command]
fn get_contents(state: State<'_, AppState>) -> Vec<ContentEntry> {
    let contents = state.contents.lock().unwrap();
    let mut entries = contents.clone();
    entries.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    entries
}

#[tauri::command]
fn add_content(
    app: AppHandle,
    state: State<'_, AppState>,
    title: Option<String>,
    text: String,
    mode: String,
) -> Result<ContentEntry, String> {
    if text.trim().is_empty() {
        return Err("内容不能为空".into());
    }
    let mode = normalize_content_mode(&mode);
    let mut contents = state.contents.lock().unwrap();
    if contents
        .iter()
        .any(|entry| entry.text == text && entry.mode == mode)
    {
        return Err("内容库中已存在相同内容".into());
    }

    let now = unix_millis();
    let entry = ContentEntry {
        id: unix_nanos().to_string(),
        title: normalize_content_title(title),
        text,
        mode,
        created_at: now,
        updated_at: now,
    };
    contents.push(entry.clone());
    save_contents_to_file(&app, contents.as_slice())?;
    drop(contents);
    let _ = app.emit_to("content", "reload-content", ());
    Ok(entry)
}

#[tauri::command]
fn update_content(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    title: Option<String>,
    text: String,
    mode: String,
) -> Result<(), String> {
    if text.trim().is_empty() {
        return Err("内容不能为空".into());
    }
    let mode = normalize_content_mode(&mode);
    let mut contents = state.contents.lock().unwrap();
    if contents
        .iter()
        .any(|entry| entry.id != id && entry.text == text && entry.mode == mode)
    {
        return Err("内容库中已存在相同内容".into());
    }
    let entry = contents
        .iter_mut()
        .find(|entry| entry.id == id)
        .ok_or_else(|| "内容不存在".to_string())?;
    entry.title = normalize_content_title(title);
    entry.text = text;
    entry.mode = mode;
    entry.updated_at = unix_millis();
    save_contents_to_file(&app, contents.as_slice())?;
    drop(contents);
    let _ = app.emit_to("content", "reload-content", ());
    Ok(())
}

#[tauri::command]
fn delete_content(app: AppHandle, state: State<'_, AppState>, id: String) -> Result<(), String> {
    let mut contents = state.contents.lock().unwrap();
    let original_len = contents.len();
    contents.retain(|entry| entry.id != id);
    if contents.len() == original_len {
        return Err("内容不存在".into());
    }
    save_contents_to_file(&app, contents.as_slice())?;
    drop(contents);
    let _ = app.emit_to("content", "reload-content", ());
    Ok(())
}

#[tauri::command]
fn open_content(app: AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
    let mut prev = state.previous_app_pid.lock().unwrap();
    if prev.is_none() {
        let pid = platform::get_frontmost_pid();
        if pid > 0 && pid != std::process::id() as i32 {
            *prev = Some(pid);
        }
    }
    drop(prev);

    if let Some(win) = app.get_webview_window("content") {
        let _ = win.center();
        let _ = win.show();
        let _ = win.set_focus();
    }
    let _ = app.emit_to("content", "reload-content", ());
    Ok(())
}

#[tauri::command]
fn paste_content(app: AppHandle, state: State<'_, AppState>, id: String) -> Result<(), String> {
    let lease = ConfirmationLease::acquire(state.paste_in_progress.clone())?;
    let entry = state
        .contents
        .lock()
        .unwrap()
        .iter()
        .find(|entry| entry.id == id)
        .cloned()
        .ok_or_else(|| "内容不存在".to_string())?;
    if !platform::can_simulate_paste() {
        write_paste_diag("ax_trusted=false target_pid=0 front_after_activate=0 front_after_paste=0 result=permission_denied");
        return Err("QuickInput 没有辅助功能权限，请在系统设置中授权 /Applications/QuickInput.app 后重启应用".into());
    }

    let target_pid = state
        .previous_app_pid
        .lock()
        .unwrap()
        .as_ref()
        .copied()
        .ok_or_else(|| "未找到目标应用，请重新打开内容库后再试".to_string())?;
    if target_pid <= 0 {
        return Err("未找到目标应用，请重新打开内容库后再试".into());
    }

    let settings = state.settings.lock().unwrap().clone();
    let previous_clipboard = clipboard_snapshot();
    clipboard_set_for_mode(&entry.text, &entry.mode, None)
        .map_err(|e| format!("clipboard: {}", e))?;
    state.previous_app_pid.lock().unwrap().take();

    if let Some(win) = app.get_webview_window("content") {
        let _ = win.hide();
    }

    let app_clone = app.clone();
    let content_id = entry.id.clone();
    std::thread::spawn(move || {
        let _lease = lease;
        std::thread::sleep(Duration::from_millis(400));
        platform::activate_pid(target_pid);
        std::thread::sleep(Duration::from_millis(150));

        let front_after_activate = platform::get_frontmost_pid();
        let paste_result = platform::simulate_paste();
        std::thread::sleep(Duration::from_millis(80));
        let front_after_paste = platform::get_frontmost_pid();
        let result_label = match &paste_result {
            Ok(_) => "ok".to_string(),
            Err(err) => format!("error:{}", err.replace('\n', " ")),
        };
        write_paste_diag(&format!(
            "content_paste ax_trusted=true target_pid={} front_after_activate={} front_after_paste={} result={}",
            target_pid, front_after_activate, front_after_paste, result_label
        ));

        std::thread::sleep(Duration::from_millis(400));
        if settings.restore_clipboard {
            let _ = restore_clipboard_snapshot(previous_clipboard);
        }

        if let Err(message) = paste_result {
            let state = app_clone.state::<AppState>();
            *state.previous_app_pid.lock().unwrap() = Some(target_pid);
            let _ = app_clone.emit_to(
                "content",
                "content-paste-error",
                serde_json::json!({ "content_id": content_id, "message": message }),
            );
            if let Some(win) = app_clone.get_webview_window("content") {
                let _ = win.show();
                let _ = win.set_focus();
            }
        }
    });
    Ok(())
}

fn unix_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

fn unix_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0)
}

fn normalize_content_title(title: Option<String>) -> Option<String> {
    title
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn content_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("content.json"))
}

fn legacy_history_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    Ok(dir.join("history.json"))
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
struct LegacyHistoryEntry {
    id: String,
    text: String,
    #[serde(default = "default_plain_mode")]
    mode: String,
    #[serde(default = "default_legacy_kind")]
    kind: String,
    #[serde(default)]
    created_at: String,
}

fn default_plain_mode() -> String {
    "plain".into()
}

fn default_legacy_kind() -> String {
    "history".into()
}

fn legacy_timestamp_to_millis(value: &str) -> u64 {
    let timestamp = value.parse::<u64>().unwrap_or(0);
    if timestamp >= 1_000_000_000_000_000 {
        timestamp / 1_000_000
    } else if timestamp >= 1_000_000_000_000 {
        timestamp
    } else if timestamp > 0 {
        timestamp * 1000
    } else {
        unix_millis()
    }
}

fn load_contents(app: &AppHandle) -> Vec<ContentEntry> {
    let path = match content_path(app) {
        Ok(path) => path,
        Err(_) => return Vec::new(),
    };
    if let Ok(content) = std::fs::read_to_string(&path) {
        return serde_json::from_str::<Vec<ContentEntry>>(&content).unwrap_or_default();
    }

    let legacy_path = match legacy_history_path(app) {
        Ok(path) => path,
        Err(_) => return Vec::new(),
    };
    let Ok(legacy_content) = std::fs::read_to_string(legacy_path) else {
        return Vec::new();
    };
    let legacy_entries: Vec<LegacyHistoryEntry> =
        serde_json::from_str(&legacy_content).unwrap_or_default();
    let contents = migrate_legacy_entries(legacy_entries);
    let _ = save_contents_to_file(app, &contents);
    contents
}

fn migrate_legacy_entries(legacy_entries: Vec<LegacyHistoryEntry>) -> Vec<ContentEntry> {
    legacy_entries
        .into_iter()
        .filter(|entry| entry.kind == "snippet")
        .map(|entry| {
            let timestamp = legacy_timestamp_to_millis(&entry.created_at);
            ContentEntry {
                id: entry.id,
                title: None,
                text: entry.text,
                mode: normalize_content_mode(&entry.mode),
                created_at: timestamp,
                updated_at: timestamp,
            }
        })
        .collect()
}

fn save_contents_to_file(app: &AppHandle, contents: &[ContentEntry]) -> Result<(), String> {
    let path = content_path(app)?;
    let content = serde_json::to_string_pretty(contents).map_err(|e| e.to_string())?;
    std::fs::write(path, content).map_err(|e| e.to_string())
}

fn store_main_window_position(state: &AppState, position: tauri::PhysicalPosition<i32>) {
    let mut settings = state.settings.lock().unwrap();
    settings.window_x = Some(position.x);
    settings.window_y = Some(position.y);
}

fn persist_main_window_position(app: &AppHandle) -> Result<(), String> {
    let Some(window) = app.get_webview_window("main") else {
        return Ok(());
    };
    let Ok(position) = window.outer_position() else {
        return Ok(());
    };

    let state = app.state::<AppState>();
    let mut settings = state.settings.lock().unwrap();
    settings.window_x = Some(position.x);
    settings.window_y = Some(position.y);
    if !settings.remember_window_position {
        return Ok(());
    }

    let path = settings_path(app)?;
    let content = serde_json::to_string_pretty(&*settings).map_err(|e| e.to_string())?;
    std::fs::write(path, content).map_err(|e| e.to_string())
}

fn show_input(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        let state = app.state::<AppState>();
        let settings = state.settings.lock().unwrap();
        let position = settings.window_position.clone();
        let remember = settings.remember_window_position;
        let width = settings.window_width;
        let height = settings.window_height;
        let saved_x = settings.window_x;
        let saved_y = settings.window_y;
        drop(settings);

        let _ = win.set_size(LogicalSize::new(width, height));

        if remember && position != "cursor" {
            let saved_position_visible = match (saved_x, saved_y) {
                (Some(x), Some(y)) => win
                    .available_monitors()
                    .map(|monitors| {
                        monitors.iter().any(|monitor| {
                            let area = monitor.work_area();
                            x >= area.position.x
                                && y >= area.position.y
                                && x < area.position.x + area.size.width as i32
                                && y < area.position.y + area.size.height as i32
                        })
                    })
                    .unwrap_or(false),
                _ => false,
            };
            if saved_position_visible {
                let position = tauri::PhysicalPosition::new(saved_x.unwrap(), saved_y.unwrap());
                let _ = win.set_position(Position::Physical(position));
            } else {
                let _ = win.center();
            }
        } else if position == "cursor" {
            if let Ok(pos) = win.cursor_position() {
                let physical = tauri::PhysicalPosition::new(pos.x as i32, pos.y as i32);
                let _ = win.set_position(Position::Physical(physical));
            }
        } else {
            let _ = win.center();
        }
        let _ = win.show();
        let _ = win.unminimize();
        let _ = win.set_focus();
    }
}

fn validate_hotkey_conflicts(settings: &Settings) -> Result<(), String> {
    let mut hotkeys = vec![
        ("唤起", &settings.trigger_hotkey),
        ("内容库", &settings.content_hotkey),
        ("设置", &settings.settings_hotkey),
        ("确认", &settings.confirm_hotkey),
        ("取消", &settings.cancel_hotkey),
    ];
    let send_hotkey = "Shift+Enter".to_string();
    if settings.paste_and_send_enabled {
        hotkeys.push(("粘贴并发送", &send_hotkey));
    }

    let mut seen: Vec<(&str, String, Shortcut)> = Vec::new();
    for (label, raw) in hotkeys {
        let value = raw.trim();
        if value.is_empty() {
            return Err(format!("{}快捷键不能为空", label));
        }
        let shortcut = Shortcut::from_str(value)
            .map_err(|e| format!("无效的{}快捷键 {}: {}", label, value, e))?;
        if let Some((existing_label, _, _)) =
            seen.iter().find(|(_, existing, existing_shortcut)| {
                existing.eq_ignore_ascii_case(value) || *existing_shortcut == shortcut
            })
        {
            return Err(format!(
                "{}快捷键与{}快捷键重复: {}",
                label, existing_label, value
            ));
        }
        seen.push((label, value.to_string(), shortcut));
    }

    Ok(())
}

fn apply_hotkeys(app: &AppHandle, settings: &Settings) -> Result<(), String> {
    let state = app.state::<AppState>();
    let hotkeys = [
        ("唤起", &settings.trigger_hotkey),
        ("内容库", &settings.content_hotkey),
        ("设置", &settings.settings_hotkey),
    ];

    let mut shortcuts: Vec<(&str, String, Shortcut)> = Vec::new();
    for (label, raw) in hotkeys {
        if shortcuts.iter().any(|(_, existing, _)| existing == raw) {
            return Err(format!("{}快捷键重复: {}", label, raw));
        }
        let shortcut =
            Shortcut::from_str(raw).map_err(|e| format!("无效的{}快捷键 {}: {}", label, raw, e))?;
        shortcuts.push((label, raw.clone(), shortcut));
    }

    let old = state.registered_hotkeys.lock().unwrap().clone();
    let desired: Vec<String> = shortcuts.iter().map(|(_, raw, _)| raw.clone()).collect();
    if old == desired {
        return Ok(());
    }
    for old_hotkey in &old {
        if let Ok(shortcut) = Shortcut::from_str(old_hotkey) {
            let _ = app.global_shortcut().unregister(shortcut);
        }
    }
    state.registered_hotkeys.lock().unwrap().clear();

    let mut registered: Vec<String> = Vec::new();

    for (label, raw, shortcut) in shortcuts {
        if let Err(err) = app.global_shortcut().register(shortcut) {
            for registered_hotkey in &registered {
                if let Ok(shortcut) = Shortcut::from_str(registered_hotkey) {
                    let _ = app.global_shortcut().unregister(shortcut);
                }
            }
            for old_hotkey in &old {
                if let Ok(shortcut) = Shortcut::from_str(old_hotkey) {
                    let _ = app.global_shortcut().register(shortcut);
                }
            }
            *state.registered_hotkeys.lock().unwrap() = old;
            return Err(format!("{}快捷键注册失败 {}: {}", label, raw, err));
        }
        registered.push(raw);
    }

    *state.registered_hotkeys.lock().unwrap() = registered;

    Ok(())
}

fn apply_autostart(app: &AppHandle, enabled: bool) -> Result<(), String> {
    let autostart = app.autolaunch();
    let currently_enabled = autostart
        .is_enabled()
        .map_err(|e| format!("读取开机自启状态失败: {}", e))?;
    if currently_enabled == enabled {
        return Ok(());
    }

    if enabled {
        autostart
            .enable()
            .map_err(|e| format!("开启开机自启失败: {}", e))
    } else {
        autostart
            .disable()
            .map_err(|e| format!("关闭开机自启失败: {}", e))
    }
}

fn settings_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("settings.json"))
}

fn load_settings(app: &AppHandle) -> Settings {
    let path = match settings_path(app) {
        Ok(p) => p,
        Err(_) => return Settings::default(),
    };
    match std::fs::read_to_string(path) {
        Ok(content) => serde_json::from_str(&content)
            .map(|mut settings: Settings| {
                normalize_settings(&mut settings);
                settings
            })
            .unwrap_or_default(),
        Err(_) => Settings::default(),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        let settings = app.state::<AppState>().settings.lock().unwrap().clone();
                        let shortcut = *shortcut;
                        let app = app.clone();
                        let app_inner = app.clone();

                        let _ = app.run_on_main_thread(move || {
                            // Capture frontmost PID on the main thread BEFORE show_input switches focus
                            let front_pid = platform::get_frontmost_pid();
                            if let Ok(trigger) = Shortcut::from_str(&settings.trigger_hotkey) {
                                if shortcut == trigger {
                                    if front_pid > 0 {
                                        *app_inner
                                            .state::<AppState>()
                                            .previous_app_pid
                                            .lock()
                                            .unwrap() = Some(front_pid);
                                    }
                                    show_input(&app_inner);
                                    return;
                                }
                            }
                            if let Ok(content) = Shortcut::from_str(&settings.content_hotkey) {
                                if shortcut == content {
                                    let _ = open_content(app_inner.clone());
                                    return;
                                }
                            }
                            if let Ok(settings_shortcut) =
                                Shortcut::from_str(&settings.settings_hotkey)
                            {
                                if shortcut == settings_shortcut {
                                    let _ = open_settings(app_inner.clone());
                                    return;
                                }
                            }
                        });
                    }
                })
                .build(),
        )
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            // A second instance tried to launch; focus the main window on the first instance.
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            get_settings,
            save_settings,
            confirm_input,
            cancel_input,
            open_settings,
            get_contents,
            add_content,
            update_content,
            delete_content,
            open_content,
            paste_content
        ])
        .setup(|app| {
            #[cfg(target_os = "macos")]
            {
                let _ = app
                    .handle()
                    .set_activation_policy(tauri::ActivationPolicy::Accessory);
            }
            let state = app.state::<AppState>();
            let settings = load_settings(app.handle());
            *state.settings.lock().unwrap() = settings.clone();
            *state.contents.lock().unwrap() = load_contents(app.handle());
            apply_hotkeys(app.handle(), &settings)?;
            setup_tray(app.handle())?;
            Ok(())
        })
        .on_window_event(|window, event| match event {
            WindowEvent::Moved(position) if window.label() == "main" => {
                store_main_window_position(&window.state::<AppState>(), *position);
            }
            WindowEvent::Focused(false) if window.label() == "main" => {
                let _ = persist_main_window_position(window.app_handle());
            }
            WindowEvent::CloseRequested { api, .. }
                if window.label() == "settings" || window.label() == "content" =>
            {
                api.prevent_close();
                let _ = window.hide();
            }
            WindowEvent::CloseRequested { api, .. } if window.label() == "main" => {
                api.prevent_close();
                let _ = persist_main_window_position(window.app_handle());
                let _ = window.hide();
            }
            _ => {}
        })
        .run(tauri::generate_context!())
        .expect("error while running QuickInput");
}

fn clipboard_set_text(text: &str) -> Result<(), String> {
    let mut c = arboard::Clipboard::new().map_err(|e| e.to_string())?;
    c.set_text(text.to_string()).map_err(|e| e.to_string())?;
    Ok(())
}

fn clipboard_snapshot() -> ClipboardSnapshot {
    let mut c = match arboard::Clipboard::new() {
        Ok(c) => c,
        Err(_) => {
            return ClipboardSnapshot {
                text: None,
                html: None,
            }
        }
    };
    let text = c.get_text().ok();
    let html = c.get().html().ok();
    ClipboardSnapshot { text, html }
}

fn restore_clipboard_snapshot(snapshot: ClipboardSnapshot) -> Result<(), String> {
    match (snapshot.html, snapshot.text) {
        (Some(html), Some(text)) => clipboard_set_html(&html, &text),
        (Some(html), None) => clipboard_set_html(&html, ""),
        (None, Some(text)) => clipboard_set_text(&text),
        (None, None) => Ok(()),
    }
}

fn clipboard_set_for_mode(
    text: &str,
    mode: &str,
    rendered_html: Option<String>,
) -> Result<(), String> {
    if normalize_content_mode(mode) == "markdown" {
        // Use frontend-rendered HTML if available; fall back to Rust renderer
        let html = rendered_html.unwrap_or_else(|| markdown_to_html(text));
        let wrapped = if html.contains("<html") {
            html
        } else {
            format!("<html><body>{}</body></html>", html)
        };
        clipboard_set_html(&wrapped, text)
    } else {
        clipboard_set_text(text)
    }
}

fn clipboard_set_html(html: &str, alt_text: &str) -> Result<(), String> {
    let mut c = arboard::Clipboard::new().map_err(|e| e.to_string())?;
    c.set_html(html.to_string(), Some(alt_text.to_string()))
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn markdown_to_html(text: &str) -> String {
    let mut html = String::from("<html><body>");
    let mut paragraph = String::new();
    let mut in_code = false;
    let mut list_open = false;

    for raw in text.lines() {
        let line = raw.trim_end();
        if line.trim_start().starts_with("```") {
            flush_paragraph(&mut html, &mut paragraph);
            if list_open {
                html.push_str("</ul>");
                list_open = false;
            }
            if in_code {
                html.push_str("</code></pre>");
                in_code = false;
            } else {
                html.push_str("<pre><code>");
                in_code = true;
            }
            continue;
        }

        if in_code {
            html.push_str(&escape_html(line));
            html.push('\n');
            continue;
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            flush_paragraph(&mut html, &mut paragraph);
            if list_open {
                html.push_str("</ul>");
                list_open = false;
            }
            continue;
        }

        if let Some((level, heading)) = markdown_heading(trimmed) {
            flush_paragraph(&mut html, &mut paragraph);
            if list_open {
                html.push_str("</ul>");
                list_open = false;
            }
            html.push_str(&format!(
                "<h{}>{}</h{}>",
                level,
                inline_markdown(heading),
                level
            ));
            continue;
        }

        if let Some(item) = trimmed
            .strip_prefix("- ")
            .or_else(|| trimmed.strip_prefix("* "))
        {
            flush_paragraph(&mut html, &mut paragraph);
            if !list_open {
                html.push_str("<ul>");
                list_open = true;
            }
            html.push_str(&format!("<li>{}</li>", inline_markdown(item)));
            continue;
        }

        if !paragraph.is_empty() {
            paragraph.push(' ');
        }
        paragraph.push_str(&inline_markdown(trimmed));
    }

    flush_paragraph(&mut html, &mut paragraph);
    if list_open {
        html.push_str("</ul>");
    }
    if in_code {
        html.push_str("</code></pre>");
    }
    html.push_str("</body></html>");
    html
}

fn markdown_heading(line: &str) -> Option<(usize, &str)> {
    let level = line.chars().take_while(|ch| *ch == '#').count();
    if (1..=6).contains(&level) && line.chars().nth(level) == Some(' ') {
        Some((level, line[level + 1..].trim()))
    } else {
        None
    }
}

fn flush_paragraph(html: &mut String, paragraph: &mut String) {
    if !paragraph.is_empty() {
        html.push_str("<p>");
        html.push_str(paragraph);
        html.push_str("</p>");
        paragraph.clear();
    }
}

fn inline_markdown(text: &str) -> String {
    let escaped = escape_html(text);
    let with_code = replace_pairs(&escaped, "`", "<code>", "</code>");
    let with_strong = replace_pairs(&with_code, "**", "<strong>", "</strong>");
    let with_emphasis = replace_pairs(&with_strong, "*", "<em>", "</em>");
    replace_markdown_links(&with_emphasis)
}

fn replace_markdown_links(input: &str) -> String {
    let mut rest = input;
    let mut out = String::new();

    while let Some(start) = rest.find('[') {
        out.push_str(&rest[..start]);
        let after_open = &rest[start + 1..];
        let Some(label_end) = after_open.find("](") else {
            out.push_str(&rest[start..]);
            return out;
        };
        let label = &after_open[..label_end];
        let after_url_open = &after_open[label_end + 2..];
        let Some(url_end) = after_url_open.find(')') else {
            out.push_str(&rest[start..]);
            return out;
        };
        let url = &after_url_open[..url_end];

        if let Some(safe_url) = sanitize_link_url(url) {
            out.push_str(&format!("<a href=\"{}\">{}</a>", safe_url, label));
        } else {
            out.push_str(label);
        }

        rest = &after_url_open[url_end + 1..];
    }

    out.push_str(rest);
    out
}

fn sanitize_link_url(url: &str) -> Option<String> {
    let trimmed = url.trim().replace("&quot;", "").replace("&#39;", "");
    let lower = trimmed.to_ascii_lowercase();
    if (lower.starts_with("http://")
        || lower.starts_with("https://")
        || lower.starts_with("mailto:"))
        && !trimmed
            .chars()
            .any(|ch| ch.is_control() || ch == '<' || ch == '>')
    {
        Some(trimmed)
    } else {
        None
    }
}

fn replace_pairs(input: &str, marker: &str, open: &str, close: &str) -> String {
    let mut rest = input;
    let mut out = String::new();
    let mut is_open = true;
    while let Some(pos) = rest.find(marker) {
        out.push_str(&rest[..pos]);
        out.push_str(if is_open { open } else { close });
        rest = &rest[pos + marker.len()..];
        is_open = !is_open;
    }
    out.push_str(rest);
    out
}

fn escape_html(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn setup_tray(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let open = MenuItemBuilder::new("打开输入框").id("show").build(app)?;
    let content = MenuItemBuilder::new("内容库").id("content").build(app)?;
    let settings = MenuItemBuilder::new("设置...").id("settings").build(app)?;
    let quit = MenuItemBuilder::new("退出").id("quit").build(app)?;

    let menu = MenuBuilder::new(app)
        .item(&open)
        .item(&content)
        .separator()
        .item(&settings)
        .separator()
        .item(&quit)
        .build()?;

    let img = image::load_from_memory(include_bytes!("../icons/32x32.png"))
        .map_err(|e| format!("failed to decode icon: {}", e))?
        .into_rgba8();
    let (width, height) = img.dimensions();
    let icon = tauri::image::Image::new_owned(img.into_raw(), width, height);

    TrayIconBuilder::new()
        .icon(icon)
        .menu(&menu)
        .tooltip("QuickInput - 全局快速输入")
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show" => {
                let pid = platform::get_frontmost_pid();
                if pid > 0 {
                    *app.state::<AppState>().previous_app_pid.lock().unwrap() = Some(pid);
                }
                show_input(app);
            }
            "content" => {
                let _ = open_content(app.clone());
            }
            "settings" => {
                if let Some(win) = app.get_webview_window("settings") {
                    let _ = win.center();
                    let _ = win.show();
                    let _ = win.set_focus();
                }
            }
            "quit" => {
                let _ = persist_main_window_position(app);
                app.exit(0);
            }
            _ => {}
        })
        .build(app)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn send_shortcut_conflicts_are_checked_only_when_enabled() {
        for field in [
            "trigger_hotkey",
            "content_hotkey",
            "settings_hotkey",
            "confirm_hotkey",
            "cancel_hotkey",
        ] {
            let mut json = serde_json::to_value(Settings::default()).unwrap();
            json[field] = serde_json::json!("Shift+Enter");
            let mut settings: Settings = serde_json::from_value(json).unwrap();
            assert!(validate_hotkey_conflicts(&settings).is_ok());
            settings.paste_and_send_enabled = true;
            assert!(validate_hotkey_conflicts(&settings).is_err());
        }
    }

    #[test]
    fn send_setting_round_trips() {
        let mut settings = Settings::default();
        settings.paste_and_send_enabled = true;
        let loaded: Settings =
            serde_json::from_value(serde_json::to_value(settings).unwrap()).unwrap();
        assert!(loaded.paste_and_send_enabled);
        assert!(validate_hotkey_conflicts(&loaded).is_ok());
    }
    #[test]
    fn settings_defaults_include_confirmed_input_preferences() {
        let settings = Settings::default();

        assert_eq!(settings.pure_mode, false);
        assert!(!settings.paste_and_send_enabled);
        assert_eq!(settings.input_font_size, 14);
        assert_eq!(settings.window_width, 720);
        assert_eq!(settings.window_height, 240);
        assert_eq!(settings.window_x, None);
        assert_eq!(settings.window_y, None);
        assert_eq!(settings.content_hotkey, "Alt+R");
    }

    #[test]
    fn input_font_size_is_normalized_to_supported_range() {
        assert_eq!(normalize_input_font_size(12), 12);
        assert_eq!(normalize_input_font_size(24), 24);
        assert_eq!(normalize_input_font_size(32), 32);
        assert_eq!(normalize_input_font_size(11), 14);
        assert_eq!(normalize_input_font_size(33), 14);
    }

    #[test]
    fn legacy_settings_json_uses_confirmed_input_defaults() {
        let settings: Settings = serde_json::from_value(serde_json::json!({
            "trigger_hotkey": "Alt+Space",
            "confirm_hotkey": "Alt+Enter",
            "window_position": "center",
            "default_mode": "plain",
            "autostart": false
        }))
        .unwrap();

        assert_eq!(settings.pure_mode, false);
        assert!(!settings.paste_and_send_enabled);
        assert_eq!(settings.input_font_size, 14);
        assert_eq!(settings.window_width, 720);
        assert_eq!(settings.window_height, 240);
        assert_eq!(settings.window_x, None);
        assert_eq!(settings.window_y, None);
        assert_eq!(settings.content_hotkey, "Alt+R");
    }

    #[test]
    fn legacy_default_window_and_code_mode_are_normalized() {
        let mut settings: Settings = serde_json::from_value(serde_json::json!({
            "trigger_hotkey": "Alt+Space",
            "confirm_hotkey": "Alt+Enter",
            "window_position": "center",
            "window_width": 600,
            "window_height": 400,
            "default_mode": "code",
            "autostart": false
        }))
        .unwrap();

        normalize_settings(&mut settings);

        assert_eq!(settings.window_width, 720);
        assert_eq!(settings.window_height, 240);
        assert_eq!(settings.default_mode, "plain");
    }

    #[test]
    fn window_and_opacity_settings_are_clamped() {
        let mut settings = Settings::default();
        settings.window_width = 100;
        settings.window_height = 1000;
        settings.bg_opacity = 0.01;
        settings.window_position = "unknown".into();

        normalize_settings(&mut settings);

        assert_eq!(settings.window_width, 400);
        assert_eq!(settings.window_height, 800);
        assert_eq!(settings.bg_opacity, 0.1);
        assert_eq!(settings.window_position, "center");
    }

    #[test]
    fn custom_window_size_is_preserved() {
        let mut settings = Settings::default();
        settings.window_width = 600;
        settings.window_height = 500;

        normalize_settings(&mut settings);

        assert_eq!(settings.window_width, 600);
        assert_eq!(settings.window_height, 500);
    }

    #[test]
    fn default_hotkeys_are_valid_and_unique() {
        assert!(validate_hotkey_conflicts(&Settings::default()).is_ok());
    }

    #[test]
    fn duplicate_hotkeys_are_rejected() {
        let mut settings = Settings::default();
        settings.confirm_hotkey = settings.trigger_hotkey.clone();

        assert!(validate_hotkey_conflicts(&settings).is_err());
    }

    #[test]
    fn window_coordinates_are_persisted_in_settings() {
        let mut settings = Settings::default();
        settings.window_x = Some(320);
        settings.window_y = Some(180);

        let value = serde_json::to_value(settings).unwrap();

        assert_eq!(value["window_x"], 320);
        assert_eq!(value["window_y"], 180);
    }

    #[test]
    fn legacy_history_hotkey_is_migrated() {
        let settings: Settings = serde_json::from_value(serde_json::json!({
            "trigger_hotkey": "Alt+Space",
            "history_hotkey": "Alt+H",
            "confirm_hotkey": "Alt+Enter",
            "window_position": "center",
            "default_mode": "plain",
            "autostart": false
        }))
        .unwrap();

        assert_eq!(settings.content_hotkey, "Alt+H");
    }

    #[test]
    fn only_legacy_snippets_are_migrated_to_content_library() {
        let entries = vec![
            LegacyHistoryEntry {
                id: "history-1".into(),
                text: "auto history".into(),
                mode: "plain".into(),
                kind: "history".into(),
                created_at: "1760000000000000000".into(),
            },
            LegacyHistoryEntry {
                id: "snippet-1".into(),
                text: "important command".into(),
                mode: "code".into(),
                kind: "snippet".into(),
                created_at: "1760000000000000000".into(),
            },
        ];

        let contents = migrate_legacy_entries(entries);

        assert_eq!(contents.len(), 1);
        assert_eq!(contents[0].id, "snippet-1");
        assert_eq!(contents[0].text, "important command");
        assert_eq!(contents[0].mode, "plain");
        assert_eq!(contents[0].created_at, 1_760_000_000_000);
        assert_eq!(contents[0].updated_at, 1_760_000_000_000);
    }
}
