/*

Copyright (c) 2026-present, RoRvzzz. All rights reserved.

https://rorvzzz.cool

*/
#![allow(non_snake_case)]

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use Ruststrap_core::{
    decode_watcher_data, do_install, do_uninstall, do_uninstall_for_reinstall, encode_watcher_data,
    execute_bootstrap, installed_app_path, launch_trayhost_process, launch_watcher_process,
    region_selector_datacenters as core_region_selector_datacenters,
    region_selector_join as core_region_selector_join,
    region_selector_search_games as core_region_selector_search_games,
    region_selector_servers as core_region_selector_servers,
    region_selector_status as core_region_selector_status, runtime_readiness, wait_for_recent_player_log,
    BootstrapRuntime, BootstrapRuntimeConfig, CookieState, CookiesManager, DomainEvent,
    FastFlagManager, FilesystemBootstrapRuntime, LaunchMode, ParsedLaunchSettings, PromptKind,
    RuntimeReadiness, Watcher, WatcherData,
};
use Ruststrap_platform_windows::{ShellBackend, WindowsShellBackend};

#[derive(Debug, Clone)]
struct RuntimeHost {
    runtime: Arc<FilesystemBootstrapRuntime>,
    shell: WindowsShellBackend,
    startup_launch: Arc<Mutex<Option<StartupLaunchRequest>>>,
}

#[derive(Debug, Clone, Serialize)]
struct EventEnvelope<T> {
    source: &'static str,
    payload: T,
}

#[derive(Debug, Clone, Serialize)]
struct StatusPayload {
    status: &'static str,
    detail: String,
}

type CommandResult = Result<(), String>;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct FullInstallResult {
    relaunched: bool,
    installed_exe_path: String,
}

#[derive(Debug, Clone)]
struct StartupLaunchRequest {
    mode: LaunchMode,
    raw_args: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct StartupLaunchPayload {
    mode: String,
    raw_args: Option<String>,
}

impl StartupLaunchPayload {
    fn from_request(request: StartupLaunchRequest) -> Self {
        let mode = match request.mode {
            LaunchMode::Player => "player",
            LaunchMode::Studio => "studio",
            LaunchMode::StudioAuth => "studio_auth",
            LaunchMode::Unknown => "unknown",
            LaunchMode::None => "none",
        }
        .to_string();

        Self {
            mode,
            raw_args: request.raw_args,
        }
    }
}

fn build_runtime() -> Result<FilesystemBootstrapRuntime, String> {
    let base_dir = runtime_base_dir();
    let config = BootstrapRuntimeConfig::from_base_dir(base_dir);
    FilesystemBootstrapRuntime::new(config).map_err(|err| err.to_string())
}

fn runtime_base_dir() -> PathBuf {
    if let Ok(override_path) = std::env::var("Ruststrap_BASE_DIR") {
        return PathBuf::from(override_path);
    }

    let local_app_data = std::env::var("LOCALAPPDATA").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(local_app_data).join("Ruststrap")
}

fn emit_status(app: &AppHandle, detail: impl Into<String>) -> CommandResult {
    app.emit(
        "bootstrap_status",
        EventEnvelope {
            source: "tauri-command",
            payload: StatusPayload {
                status: "ok",
                detail: detail.into(),
            },
        },
    )
    .map_err(|err| format!("failed to emit bootstrap_status: {err}"))?;
    Ok(())
}

fn emit_prompt(app: &AppHandle, kind: PromptKind, message: impl Into<String>) -> CommandResult {
    let kind = match kind {
        PromptKind::ConfirmLaunch => "confirm_launch",
        PromptKind::ChannelChange => "channel_change",
        PromptKind::UpdateAvailable => "update_available",
        PromptKind::InstallLocationMismatch => "install_location_mismatch",
    };

    app.emit(
        "prompt_required",
        EventEnvelope {
            source: "core-runtime",
            payload: serde_json::json!({
                "kind": kind,
                "message": message.into()
            }),
        },
    )
    .map_err(|err| format!("failed to emit prompt_required: {err}"))?;
    Ok(())
}

fn emit_domain_event(app: &AppHandle, event: DomainEvent) -> CommandResult {
    match event {
        DomainEvent::BootstrapStatus { message } => emit_status(app, message),
        DomainEvent::Progress { current, total } => app
            .emit(
                "progress",
                EventEnvelope {
                    source: "core-runtime",
                    payload: serde_json::json!({
                        "current": current,
                        "total": total
                    }),
                },
            )
            .map_err(|err| format!("failed to emit progress: {err}")),
        DomainEvent::PromptRequired { kind, message } => emit_prompt(app, kind, message),
        DomainEvent::ConnectivityError { title, description } => app
            .emit(
                "connectivity_error",
                EventEnvelope {
                    source: "core-runtime",
                    payload: serde_json::json!({
                        "title": title,
                        "description": description
                    }),
                },
            )
            .map_err(|err| format!("failed to emit connectivity_error: {err}")),
        DomainEvent::FatalError { code, message } => app
            .emit(
                "fatal_error",
                EventEnvelope {
                    source: "core-runtime",
                    payload: serde_json::json!({
                        "code": code,
                        "message": message
                    }),
                },
            )
            .map_err(|err| format!("failed to emit fatal_error: {err}")),
        DomainEvent::WatcherActivity { activity } => app
            .emit(
                "watcher_activity",
                EventEnvelope {
                    source: "core-runtime",
                    payload: activity,
                },
            )
            .map_err(|err| format!("failed to emit watcher_activity: {err}")),
    }
}

fn emit_bootstrap_result(
    app: &AppHandle,
    report: Ruststrap_core::BootstrapReport,
) -> CommandResult {
    for event in report.events {
        emit_domain_event(app, event)?;
    }
    Ok(())
}

fn build_launch_settings(mode: LaunchMode, raw_args: Option<String>) -> ParsedLaunchSettings {
    let mut args = Vec::<String>::new();
    match mode {
        LaunchMode::Player => {
            args.push("-player".to_string());
        }
        LaunchMode::Studio | LaunchMode::StudioAuth => {
            args.push("-studio".to_string());
        }
        _ => {}
    }
    if let Some(value) = raw_args.filter(|value| !value.trim().is_empty()) {
        args.push(value);
    }
    ParsedLaunchSettings::parse(&args)
}

fn launched_pid_from_report(report: &Ruststrap_core::BootstrapReport) -> u32 {
    for event in &report.events {
        if let DomainEvent::BootstrapStatus { message } = event {
            if let Some(pid_raw) = message.strip_prefix("launched_client_pid=") {
                return pid_raw.parse().unwrap_or(0);
            }
        }
    }
    0
}

fn path_eq_case_insensitive(left: &Path, right: &Path) -> bool {
    left.to_string_lossy()
        .eq_ignore_ascii_case(right.to_string_lossy().as_ref())
}

fn resolve_recent_player_log() -> String {
    wait_for_recent_player_log(
        Duration::from_secs(15),
        Duration::from_secs(30),
        Duration::from_millis(500),
    )
    .map(|path| path.to_string_lossy().to_string())
    .unwrap_or_default()
}

fn load_last_autoclose_pids(runtime: &FilesystemBootstrapRuntime) -> Vec<u32> {
    runtime
        .load_state()
        .ok()
        .and_then(|state| state.extra.get("LastAutoclosePids").cloned())
        .and_then(|value| value.as_array().cloned())
        .map(|values| {
            values
                .into_iter()
                .filter_map(|value| value.as_u64().map(|pid| pid as u32))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn start_detached_watcher(
    runtime: &FilesystemBootstrapRuntime,
    pid: u32,
    launch_mode: &str,
) -> Result<(), String> {
    if pid == 0 {
        return Ok(());
    }

    let settings = runtime.load_settings().map_err(|e| e.to_string())?;
    if !settings.enable_activity_tracking {
        return Ok(());
    }

    let current_exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let watcher_data = WatcherData {
        process_id: pid,
        log_file: resolve_recent_player_log(),
        autoclose_pids: load_last_autoclose_pids(runtime),
        handle: 0,
        launch_mode: launch_mode.to_string(),
        use_discord_rich_presence: settings.use_discord_rich_presence,
        hide_rpc_buttons: settings.hide_rpc_buttons,
        show_account_on_rich_presence: settings.show_account_on_rich_presence,
        enable_custom_status_display: settings.enable_custom_status_display,
        show_using_ruststrap_rpc: settings.show_using_ruststrap_rpc,
        show_server_details: settings.show_server_details,
        show_server_uptime: settings.show_server_uptime,
        playtime_counter: settings.playtime_counter,
        auto_rejoin: settings.auto_rejoin,
        use_disable_app_patch: settings.use_disable_app_patch,
    };
    let encoded = encode_watcher_data(&watcher_data).map_err(|e| e.to_string())?;
    launch_watcher_process(&current_exe, &encoded).map_err(|e| e.to_string())?;
    if launch_mode.eq_ignore_ascii_case("player") {
        if let Err(error) = launch_trayhost_process(&current_exe, &encoded) {
            eprintln!("trayhost launch failed: {error}");
        }
    }
    runtime
        .set_watcher_running(true)
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn run_watcher_foreground(
    runtime: &FilesystemBootstrapRuntime,
    encoded: &str,
) -> Result<(), String> {
    let data = decode_watcher_data(encoded).map_err(|e| e.to_string())?;
    let mut watcher = Watcher::new(data);
    let result = watcher.run().map_err(|e| e.to_string());
    let _ = runtime.set_watcher_running(false);
    result
}

fn resolve_tray_icon_path() -> Result<PathBuf, String> {
    const TRAY_ICON_BYTES: &[u8] = include_bytes!("../icons/icon.ico");
    let icon_path = std::env::temp_dir().join("ruststrap_tray_icon.ico");

    let should_write = fs::metadata(&icon_path)
        .map(|meta| meta.len() != TRAY_ICON_BYTES.len() as u64)
        .unwrap_or(true);

    if should_write {
        fs::write(&icon_path, TRAY_ICON_BYTES)
            .map_err(|e| format!("failed to write tray icon cache: {e}"))?;
    }

    Ok(icon_path)
}

fn run_trayhost_foreground(encoded: &str) -> Result<(), String> {
    let data = decode_watcher_data(encoded).map_err(|e| e.to_string())?;
    if data.process_id == 0 {
        return Ok(());
    }

    #[cfg(windows)]
    let (tray_icon, open_settings_item, exit_tray_item) = {
        let icon_path = resolve_tray_icon_path()?;
        let icon = tray_icon::Icon::from_path(&icon_path, Some((32, 32)))
            .or_else(|_| tray_icon::Icon::from_path(&icon_path, Some((16, 16))))
            .or_else(|_| tray_icon::Icon::from_path(&icon_path, None))
            .map_err(|e| format!("failed to load tray icon: {e}"))?;

        let menu = tray_icon::menu::Menu::new();
        let open_settings_item = tray_icon::menu::MenuItem::new("open ruststrap", true, None);
        let exit_tray_item = tray_icon::menu::MenuItem::new("exit tray icon", true, None);
        menu.append_items(&[&open_settings_item, &exit_tray_item])
            .map_err(|e| format!("failed to build tray menu: {e}"))?;

        tray_icon::TrayIconBuilder::new()
            .with_tooltip("Ruststrap")
            .with_icon(icon)
            .with_menu(Box::new(menu))
            .with_menu_on_left_click(false)
            .build()
            .map(|tray_icon| (tray_icon, open_settings_item, exit_tray_item))
            .map_err(|e| format!("failed to build tray icon: {e}"))?
    };

    loop {
        #[cfg(windows)]
        pump_windows_messages();

        #[cfg(windows)]
        while let Ok(event) = tray_icon::menu::MenuEvent::receiver().try_recv() {
            let event_id = event.id;
            if event_id == open_settings_item.id().clone() {
                launch_menu_window();
                continue;
            }
            if event_id == exit_tray_item.id().clone() {
                drop(tray_icon);
                return Ok(());
            }
        }

        #[cfg(windows)]
        while let Ok(event) = tray_icon::TrayIconEvent::receiver().try_recv() {
            if event.id() != tray_icon.id() {
                continue;
            }
            match event {
                tray_icon::TrayIconEvent::Click {
                    button: tray_icon::MouseButton::Left,
                    button_state: tray_icon::MouseButtonState::Up,
                    ..
                } => launch_menu_window(),
                tray_icon::TrayIconEvent::DoubleClick {
                    button: tray_icon::MouseButton::Left,
                    ..
                } => launch_menu_window(),
                _ => {}
            }
        }

        std::thread::sleep(Duration::from_millis(200));
        if !is_process_alive(data.process_id) {
            #[cfg(windows)]
            drop(tray_icon);
            return Ok(());
        }
    }
}

#[cfg(windows)]
fn pump_windows_messages() {
    #[repr(C)]
    struct Point {
        x: i32,
        y: i32,
    }

    #[repr(C)]
    struct Message {
        hwnd: isize,
        message: u32,
        w_param: usize,
        l_param: isize,
        time: u32,
        point: Point,
        l_private: u32,
    }

    extern "system" {
        fn PeekMessageW(
            msg: *mut Message,
            hwnd: isize,
            msg_filter_min: u32,
            msg_filter_max: u32,
            remove_msg: u32,
        ) -> i32;
        fn TranslateMessage(msg: *const Message) -> i32;
        fn DispatchMessageW(msg: *const Message) -> isize;
    }

    const PM_REMOVE: u32 = 0x0001;

    unsafe {
        let mut message = Message {
            hwnd: 0,
            message: 0,
            w_param: 0,
            l_param: 0,
            time: 0,
            point: Point { x: 0, y: 0 },
            l_private: 0,
        };

        while PeekMessageW(&mut message as *mut Message, 0, 0, 0, PM_REMOVE) != 0 {
            let _ = TranslateMessage(&message as *const Message);
            let _ = DispatchMessageW(&message as *const Message);
        }
    }
}

#[cfg(windows)]
fn is_process_alive(pid: u32) -> bool {
    extern "system" {
        fn OpenProcess(desired_access: u32, inherit_handle: i32, process_id: u32) -> isize;
        fn GetExitCodeProcess(process: isize, exit_code: *mut u32) -> i32;
        fn CloseHandle(handle: isize) -> i32;
    }

    const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;
    const STILL_ACTIVE: u32 = 259;

    if pid == 0 {
        return false;
    }

    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle == 0 {
            return false;
        }
        let mut exit_code = 0u32;
        let ok = GetExitCodeProcess(handle, &mut exit_code as *mut u32) != 0;
        let _ = CloseHandle(handle);
        ok && exit_code == STILL_ACTIVE
    }
}

#[cfg(not(windows))]
fn is_process_alive(_pid: u32) -> bool {
    false
}

fn ensure_runtime_ready_internal(
    runtime: &FilesystemBootstrapRuntime,
) -> Result<RuntimeReadiness, String> {
    let readiness = runtime_readiness(&runtime.config.base_dir).map_err(|e| e.to_string())?;
    if !readiness.install_required && !readiness.runtime_reconcile_required {
        return Ok(readiness);
    }

    let current_exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let installed_exe = installed_app_path(&runtime.config.base_dir);

    if readiness.runtime_reconcile_required {
        do_uninstall_for_reinstall(&runtime.config.base_dir, true).map_err(|e| e.to_string())?;
    }

    do_install(&runtime.config.base_dir, &current_exe, false, false, false)
        .map_err(|e| e.to_string())?;

    let mut updated = runtime_readiness(&runtime.config.base_dir).map_err(|e| e.to_string())?;

    if readiness.runtime_reconcile_required
        && !path_eq_case_insensitive(&current_exe, &installed_exe)
    {
        std::process::Command::new(&installed_exe)
            .arg("-menu")
            .spawn()
            .map_err(|err| format!("failed to relaunch installed executable: {err}"))?;
        updated.relaunched = true;
    }

    Ok(updated)
}

// section: core commands

#[tauri::command]
async fn install(app: AppHandle, host: State<'_, RuntimeHost>) -> CommandResult {
    host.runtime
        .install_layout()
        .map_err(|err| err.to_string())?;
    emit_status(&app, "install_completed")
}

#[tauri::command]
async fn uninstall(app: AppHandle, host: State<'_, RuntimeHost>) -> CommandResult {
    host.runtime
        .uninstall_layout()
        .map_err(|err| err.to_string())?;
    emit_status(&app, "uninstall_completed")
}

#[tauri::command]
async fn launch_player(
    app: AppHandle,
    host: State<'_, RuntimeHost>,
    raw_args: Option<String>,
) -> CommandResult {
    let readiness = ensure_runtime_ready_internal(host.runtime.as_ref())?;
    if readiness.relaunched {
        return Err(
            "Ruststrap relaunched into the installed runtime; retry launch there".to_string(),
        );
    }
    if readiness.install_required {
        return Err("Ruststrap runtime is not ready after automatic repair".to_string());
    }

    let settings = build_launch_settings(LaunchMode::Player, raw_args);
    let report =
        execute_bootstrap(host.runtime.as_ref(), &settings).map_err(|err| err.to_string())?;

    let pid = launched_pid_from_report(&report);
    let _ = start_detached_watcher(host.runtime.as_ref(), pid, "player");

    emit_bootstrap_result(&app, report)
}

#[tauri::command]
async fn launch_studio(
    app: AppHandle,
    host: State<'_, RuntimeHost>,
    raw_args: Option<String>,
) -> CommandResult {
    let readiness = ensure_runtime_ready_internal(host.runtime.as_ref())?;
    if readiness.relaunched {
        return Err(
            "Ruststrap relaunched into the installed runtime; retry launch there".to_string(),
        );
    }
    if readiness.install_required {
        return Err("Ruststrap runtime is not ready after automatic repair".to_string());
    }

    let settings = build_launch_settings(LaunchMode::Studio, raw_args);
    let report =
        execute_bootstrap(host.runtime.as_ref(), &settings).map_err(|err| err.to_string())?;

    let pid = launched_pid_from_report(&report);
    let _ = start_detached_watcher(host.runtime.as_ref(), pid, "studio");

    emit_bootstrap_result(&app, report)
}

#[tauri::command]
async fn open_settings(app: AppHandle, host: State<'_, RuntimeHost>) -> CommandResult {
    let settings = host
        .runtime
        .load_settings()
        .map_err(|err| err.to_string())?;
    let location = host.runtime.config.settings_path.clone();
    let _ = host.shell.reveal_path(&location);

    app.emit(
        "bootstrap_status",
        EventEnvelope {
            source: "tauri-command",
            payload: StatusPayload {
                status: "ok",
                detail: format!("settings_loaded locale={}", settings.locale),
            },
        },
    )
    .map_err(|err| format!("failed to emit settings status: {err}"))?;
    Ok(())
}

#[tauri::command]
async fn open_external_url(host: State<'_, RuntimeHost>, url: String) -> CommandResult {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return Err("url cannot be empty".to_string());
    }
    host.shell.open_url(trimmed).map_err(|err| err.to_string())
}

#[tauri::command]
async fn run_background_updater(app: AppHandle, host: State<'_, RuntimeHost>) -> CommandResult {
    let args = ParsedLaunchSettings::parse(&[
        "-player".to_string(),
        "-nolaunch".to_string(),
        "-quiet".to_string(),
    ]);
    let report = execute_bootstrap(host.runtime.as_ref(), &args).map_err(|err| err.to_string())?;
    emit_bootstrap_result(&app, report)
}

#[tauri::command]
async fn check_updates(app: AppHandle, host: State<'_, RuntimeHost>) -> CommandResult {
    let update = host
        .runtime
        .check_updates_for_mode(LaunchMode::Player)
        .map_err(|err| err.to_string())?;

    emit_prompt(
        &app,
        PromptKind::UpdateAvailable,
        format!(
            "version={} guid={} behind_default={}",
            update.version, update.version_guid, update.is_behind_default_channel
        ),
    )
}

#[tauri::command]
async fn apply_modifications(app: AppHandle, host: State<'_, RuntimeHost>) -> CommandResult {
    let Some(version_guid) = host
        .runtime
        .current_version_for_mode(LaunchMode::Player)
        .map_err(|err| err.to_string())?
    else {
        return Err("no installed player version found".to_string());
    };

    host.runtime
        .apply_modifications(&version_guid)
        .map_err(|err| err.to_string())?;
    host.runtime
        .register_system_state(LaunchMode::Player, &version_guid)
        .map_err(|err| err.to_string())?;
    emit_status(
        &app,
        format!("modifications_applied version={version_guid}"),
    )
}

// section: new commands

#[tauri::command]
async fn get_settings(host: State<'_, RuntimeHost>) -> Result<serde_json::Value, String> {
    let settings = host
        .runtime
        .load_settings()
        .map_err(|err| err.to_string())?;
    serde_json::to_value(&settings).map_err(|err| err.to_string())
}

#[tauri::command]
async fn save_settings(host: State<'_, RuntimeHost>, settings_json: String) -> CommandResult {
    let settings: Ruststrap_core::SettingsFileCompat =
        serde_json::from_str(&settings_json).map_err(|err| err.to_string())?;
    host.runtime
        .save_settings(&settings)
        .map_err(|err| err.to_string())?;
    Ok(())
}

#[tauri::command]
async fn get_fast_flags(host: State<'_, RuntimeHost>) -> Result<serde_json::Value, String> {
    let mut mgr = FastFlagManager::new(&host.runtime.config.modifications_dir);
    mgr.load().map_err(|err| err.to_string())?;
    serde_json::to_value(&mgr.all_flags()).map_err(|err| err.to_string())
}

#[tauri::command]
async fn save_fast_flags(host: State<'_, RuntimeHost>, flags_json: String) -> CommandResult {
    let flags: std::collections::HashMap<String, serde_json::Value> =
        serde_json::from_str(&flags_json).map_err(|err| err.to_string())?;
    let mut mgr = FastFlagManager::new(&host.runtime.config.modifications_dir);
    mgr.replace_all(flags);
    mgr.save().map_err(|err| err.to_string())?;
    Ok(())
}

#[tauri::command]
async fn get_state(host: State<'_, RuntimeHost>) -> Result<serde_json::Value, String> {
    let state = host.runtime.load_state().map_err(|err| err.to_string())?;
    serde_json::to_value(&state).map_err(|err| err.to_string())
}

#[tauri::command]
async fn get_roblox_state(host: State<'_, RuntimeHost>) -> Result<serde_json::Value, String> {
    let state = host
        .runtime
        .load_roblox_state()
        .map_err(|err| err.to_string())?;
    serde_json::to_value(&state).map_err(|err| err.to_string())
}

#[tauri::command]
async fn get_runtime_status(host: State<'_, RuntimeHost>) -> Result<serde_json::Value, String> {
    let readiness = runtime_readiness(&host.runtime.config.base_dir).map_err(|e| e.to_string())?;
    serde_json::to_value(readiness).map_err(|e| e.to_string())
}

#[tauri::command]
async fn ensure_runtime_ready(host: State<'_, RuntimeHost>) -> Result<serde_json::Value, String> {
    let readiness = ensure_runtime_ready_internal(host.runtime.as_ref())?;
    serde_json::to_value(readiness).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cleanup_versions(app: AppHandle, host: State<'_, RuntimeHost>) -> CommandResult {
    let roblox_state = host
        .runtime
        .load_roblox_state()
        .map_err(|err| err.to_string())?;
    let settings = host
        .runtime
        .load_settings()
        .map_err(|err| err.to_string())?;

    let player_guid = if roblox_state.player.version_guid.is_empty() {
        None
    } else {
        Some(roblox_state.player.version_guid.as_str())
    };
    let studio_guid = if roblox_state.studio.version_guid.is_empty() {
        None
    } else {
        Some(roblox_state.studio.version_guid.as_str())
    };

    Ruststrap_core::cleanup_versions_folder(
        &host.runtime.config.versions_dir,
        player_guid,
        studio_guid,
        settings.static_directory,
    )
    .map_err(|err| err.to_string())?;

    emit_status(&app, "versions_cleaned")
}

#[tauri::command]
async fn check_self_update() -> Result<serde_json::Value, String> {
    let current_version = env!("CARGO_PKG_VERSION");
    let result = Ruststrap_core::check_for_updates(current_version, "Ruststrap/Ruststrap")
        .map_err(|err| err.to_string())?;

    Ok(serde_json::json!({
        "update_available": result.update_available,
        "current_version": result.current_version,
        "latest_version": result.latest_version,
        "download_url": result.download_url,
    }))
}

#[tauri::command]
async fn do_full_install(
    host: State<'_, RuntimeHost>,
    create_desktop_shortcut: bool,
    create_start_menu_shortcut: bool,
    import_from_ruststrap: bool,
) -> Result<FullInstallResult, String> {
    let current_exe = std::env::current_exe().map_err(|err| err.to_string())?;
    let installed_exe = installed_app_path(&host.runtime.config.base_dir);

    Ruststrap_core::do_install(
        &host.runtime.config.base_dir,
        &current_exe,
        create_desktop_shortcut,
        create_start_menu_shortcut,
        import_from_ruststrap,
    )
    .map_err(|err| err.to_string())?;

    let mut relaunched = false;
    if !path_eq_case_insensitive(&current_exe, &installed_exe) {
        std::process::Command::new(&installed_exe)
            .arg("-menu")
            .spawn()
            .map_err(|err| format!("failed to relaunch installed executable: {err}"))?;
        relaunched = true;
    }

    Ok(FullInstallResult {
        relaunched,
        installed_exe_path: installed_exe.to_string_lossy().to_string(),
    })
}

#[tauri::command]
async fn do_full_uninstall(
    app: AppHandle,
    host: State<'_, RuntimeHost>,
    keep_data: bool,
) -> CommandResult {
    Ruststrap_core::do_uninstall(&host.runtime.config.base_dir, keep_data)
        .map_err(|err| err.to_string())?;
    emit_status(&app, "full_uninstall_completed")
}

#[tauri::command]
async fn get_cookie_state() -> Result<String, String> {
    let mut mgr = CookiesManager::new(true);
    mgr.load_cookies().map_err(|err| err.to_string())?;
    let state_str = match mgr.state() {
        CookieState::Unknown => "unknown",
        CookieState::NotAllowed => "not_allowed",
        CookieState::NotFound => "not_found",
        CookieState::Invalid => "invalid",
        CookieState::Failed => "failed",
        CookieState::Success => "success",
    };
    Ok(state_str.to_string())
}

// section: phase 2+ commands

#[tauri::command]
async fn parse_join_url(launch_command: String) -> Result<serde_json::Value, String> {
    let data = Ruststrap_core::parse_launch_command(&launch_command);
    serde_json::to_value(&data).map_err(|e| e.to_string())
}

#[tauri::command]
async fn run_cleaner_cmd(host: State<'_, RuntimeHost>) -> Result<serde_json::Value, String> {
    let config = Ruststrap_core::CleanerConfig::from_base_dir(&host.runtime.config.base_dir);
    let report = Ruststrap_core::run_cleaner(
        &config,
        Ruststrap_core::CleanerAge::OneWeek,
        &[
            "RuststrapLogs",
            "RuststrapCache",
            "RobloxLogs",
            "RobloxCache",
        ],
    )
    .map_err(|e| e.to_string())?;

    Ok(serde_json::json!({
        "deleted": report.total_deleted,
        "failed": report.total_failed,
    }))
}

#[tauri::command]
async fn query_server_location(ip: String) -> Result<String, String> {
    let location = Ruststrap_core::query_server_location(&ip)
        .map_err(|e| e.to_string())?
        .unwrap_or_else(|| "Unknown".to_string());
    Ok(location)
}

#[tauri::command]
async fn get_global_settings_preset(key: String) -> Result<Option<String>, String> {
    let roblox_dir =
        std::path::PathBuf::from(std::env::var("LOCALAPPDATA").unwrap_or_default()).join("Roblox");
    let mut mgr = Ruststrap_core::GlobalSettingsManager::new(&roblox_dir);
    mgr.load().map_err(|e| e.to_string())?;
    Ok(mgr.get_preset(&key))
}

#[tauri::command]
async fn set_global_settings_preset(key: String, value: String) -> Result<(), String> {
    let roblox_dir =
        std::path::PathBuf::from(std::env::var("LOCALAPPDATA").unwrap_or_default()).join("Roblox");
    let mut mgr = Ruststrap_core::GlobalSettingsManager::new(&roblox_dir);
    mgr.load().map_err(|e| e.to_string())?;
    mgr.set_preset(&key, &value);
    mgr.save().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
async fn check_wmf() -> Result<bool, String> {
    Ok(Ruststrap_core::check_wmf_available())
}

#[tauri::command]
async fn is_roblox_running_cmd() -> Result<bool, String> {
    Ok(Ruststrap_core::is_roblox_running())
}

#[tauri::command]
async fn apply_borderless(hwnd: isize) -> Result<(), String> {
    Ruststrap_core::apply_borderless_fullscreen(hwnd);
    Ok(())
}

#[tauri::command]
async fn set_roblox_title(hwnd: isize, title: String) -> Result<(), String> {
    Ruststrap_core::set_window_title(hwnd, &title);
    Ok(())
}

#[tauri::command]
async fn get_roblox_title(hwnd: isize) -> Result<String, String> {
    Ok(Ruststrap_core::get_window_title(hwnd))
}

#[tauri::command]
async fn fetch_universe_details_cmd(universe_id: i64) -> Result<serde_json::Value, String> {
    let details = Ruststrap_core::fetch_universe_details(universe_id).map_err(|e| e.to_string())?;
    match details {
        Some(d) => serde_json::to_value(&d).map_err(|e| e.to_string()),
        None => Ok(serde_json::Value::Null),
    }
}

#[tauri::command]
async fn region_selector_status_cmd(
    host: State<'_, RuntimeHost>,
) -> Result<serde_json::Value, String> {
    let settings = host.runtime.load_settings().map_err(|e| e.to_string())?;
    let status =
        core_region_selector_status(settings.allow_cookie_access).map_err(|e| e.to_string())?;
    serde_json::to_value(status).map_err(|e| e.to_string())
}

fn launch_menu_window() {
    let Ok(exe_path) = std::env::current_exe() else {
        return;
    };
    let _ = std::process::Command::new(exe_path).arg("-menu").spawn();
}

#[tauri::command]
async fn region_selector_datacenters_cmd() -> Result<serde_json::Value, String> {
    let datacenters = core_region_selector_datacenters().map_err(|e| e.to_string())?;
    serde_json::to_value(datacenters).map_err(|e| e.to_string())
}

#[tauri::command]
async fn region_selector_search_games_cmd(query: String) -> Result<serde_json::Value, String> {
    let results = core_region_selector_search_games(&query).map_err(|e| e.to_string())?;
    serde_json::to_value(results).map_err(|e| e.to_string())
}

#[tauri::command]
async fn region_selector_servers_cmd(
    host: State<'_, RuntimeHost>,
    place_id: i64,
    cursor: Option<String>,
    sort_order: Option<i32>,
    selected_region: Option<String>,
) -> Result<serde_json::Value, String> {
    let configured_region = selected_region
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string())
        .or_else(|| {
            host.runtime
                .load_settings()
                .ok()
                .map(|settings| settings.selected_region.trim().to_string())
                .filter(|value| !value.is_empty())
        });
    let selected_region = if let Some(value) = configured_region {
        Some(value)
    } else {
        None
    };

    let servers = core_region_selector_servers(
        place_id,
        cursor.as_deref(),
        sort_order,
        selected_region.as_deref(),
    )
    .map_err(|e| e.to_string())?;
    serde_json::to_value(servers).map_err(|e| e.to_string())
}

#[tauri::command]
async fn region_selector_join_cmd(place_id: i64, job_id: String) -> CommandResult {
    core_region_selector_join(place_id, &job_id).map_err(|e| e.to_string())
}

#[tauri::command]
async fn region_selector_status(host: State<'_, RuntimeHost>) -> Result<serde_json::Value, String> {
    region_selector_status_cmd(host).await
}

#[tauri::command]
async fn region_selector_datacenters() -> Result<serde_json::Value, String> {
    region_selector_datacenters_cmd().await
}

#[tauri::command]
async fn region_selector_search_games(query: String) -> Result<serde_json::Value, String> {
    region_selector_search_games_cmd(query).await
}

#[tauri::command]
async fn region_selector_servers(
    host: State<'_, RuntimeHost>,
    place_id: i64,
    cursor: Option<String>,
    sort_order: Option<i32>,
    selected_region: Option<String>,
) -> Result<serde_json::Value, String> {
    region_selector_servers_cmd(host, place_id, cursor, sort_order, selected_region).await
}

#[tauri::command]
async fn region_selector_join(place_id: i64, job_id: String) -> CommandResult {
    region_selector_join_cmd(place_id, job_id).await
}

#[tauri::command]
async fn get_discord_presence() -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({
        "app_id": Ruststrap_core::DiscordRichPresence::app_id(),
        "ready": true,
    }))
}

#[tauri::command]
async fn weao_exploits_statuses() -> Result<serde_json::Value, String> {
    let statuses = Ruststrap_core::weao_exploit_statuses().map_err(|e| e.to_string())?;
    serde_json::to_value(statuses).map_err(|e| e.to_string())
}

#[tauri::command]
async fn weao_exploit_status(exploit: String) -> Result<serde_json::Value, String> {
    let status = Ruststrap_core::weao_exploit_status(&exploit).map_err(|e| e.to_string())?;
    serde_json::to_value(status).map_err(|e| e.to_string())
}

#[tauri::command]
async fn weao_sunc_data(scrap: String, key: String) -> Result<serde_json::Value, String> {
    let data = Ruststrap_core::weao_sunc_data(&scrap, &key).map_err(|e| e.to_string())?;
    serde_json::to_value(data).map_err(|e| e.to_string())
}

#[tauri::command]
async fn take_startup_launch(host: State<'_, RuntimeHost>) -> Result<serde_json::Value, String> {
    let mut state = host
        .startup_launch
        .lock()
        .map_err(|_| "startup launch state is poisoned".to_string())?;
    if let Some(request) = state.take() {
        serde_json::to_value(StartupLaunchPayload::from_request(request)).map_err(|e| e.to_string())
    } else {
        Ok(serde_json::Value::Null)
    }
}

#[tauri::command]
fn win_close(window: tauri::Window) {
    let _ = window.close();
}

#[tauri::command]
fn win_minimize(window: tauri::Window) {
    let _ = window.minimize();
}

#[tauri::command]
fn win_maximize(window: tauri::Window) {
    if window.is_maximized().unwrap_or(false) {
        let _ = window.unmaximize();
    } else {
        let _ = window.maximize();
    }
}

fn run_cli_mode_if_requested(
    startup_launch: &Arc<Mutex<Option<StartupLaunchRequest>>>,
) -> Option<i32> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() {
        return None;
    }

    let parsed = ParsedLaunchSettings::parse(&args);
    let has_headless_cli_action = parsed.uninstall_flag.active
        || parsed.watcher_flag.active
        || parsed.tray_host_flag.active
        || parsed.multi_instance_watcher_flag.active
        || parsed.background_updater_flag.active;

    if parsed.menu_flag.active {
        return None;
    }

    if parsed.roblox_launch_mode != LaunchMode::None {
        let request = StartupLaunchRequest {
            mode: parsed.roblox_launch_mode,
            raw_args: if parsed.roblox_launch_args.trim().is_empty() {
                None
            } else {
                Some(parsed.roblox_launch_args.clone())
            },
        };
        if let Ok(mut launch_state) = startup_launch.lock() {
            *launch_state = Some(request);
        }
        return None;
    }

    if !has_headless_cli_action {
        return None;
    }

    let runtime = match build_runtime() {
        Ok(runtime) => runtime,
        Err(err) => {
            eprintln!("Ruststrap CLI startup failed: {err}");
            return Some(1);
        }
    };

    let result = if parsed.uninstall_flag.active {
        do_uninstall(&runtime.config.base_dir, false).map_err(|e| e.to_string())
    } else if parsed.watcher_flag.active {
        if let Some(payload) = parsed.watcher_flag.data.as_deref() {
            run_watcher_foreground(&runtime, payload)
        } else {
            Err("watcher payload is required for -watcher mode".to_string())
        }
    } else if parsed.tray_host_flag.active {
        if let Some(payload) = parsed.tray_host_flag.data.as_deref() {
            run_trayhost_foreground(payload)
        } else {
            Err("trayhost payload is required for -trayhost mode".to_string())
        }
    } else if parsed.multi_instance_watcher_flag.active {
        Ruststrap_core::multi_instance_watcher::run();
        Ok(())
    } else if parsed.background_updater_flag.active {
        let updater_args = ParsedLaunchSettings::parse(&[
            "-player".to_string(),
            "-nolaunch".to_string(),
            "-quiet".to_string(),
        ]);
        execute_bootstrap(&runtime, &updater_args)
            .map(|_| ())
            .map_err(|e| e.to_string())
    } else {
        Ok(())
    };

    match result {
        Ok(()) => Some(0),
        Err(err) => {
            eprintln!("Ruststrap CLI execution failed: {err}");
            Some(1)
        }
    }
}

pub fn run() {
    let startup_launch = Arc::new(Mutex::new(None::<StartupLaunchRequest>));
    if let Some(exit_code) = run_cli_mode_if_requested(&startup_launch) {
        std::process::exit(exit_code);
    }

    let startup_launch_for_setup = startup_launch.clone();
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            install,
            uninstall,
            launch_player,
            launch_studio,
            open_settings,
            open_external_url,
            run_background_updater,
            check_updates,
            apply_modifications,
            get_settings,
            save_settings,
            get_fast_flags,
            save_fast_flags,
            get_state,
            get_roblox_state,
            get_runtime_status,
            ensure_runtime_ready,
            cleanup_versions,
            check_self_update,
            do_full_install,
            do_full_uninstall,
            get_cookie_state,
            parse_join_url,
            run_cleaner_cmd,
            query_server_location,
            get_global_settings_preset,
            set_global_settings_preset,
            check_wmf,
            is_roblox_running_cmd,
            apply_borderless,
            set_roblox_title,
            get_roblox_title,
            fetch_universe_details_cmd,
            get_discord_presence,
            weao_exploits_statuses,
            weao_exploit_status,
            weao_sunc_data,
            take_startup_launch,
            region_selector_status_cmd,
            region_selector_datacenters_cmd,
            region_selector_search_games_cmd,
            region_selector_servers_cmd,
            region_selector_join_cmd,
            region_selector_status,
            region_selector_datacenters,
            region_selector_search_games,
            region_selector_servers,
            region_selector_join,
            win_close,
            win_minimize,
            win_maximize,
        ])
        .setup(move |app| {
            let runtime = build_runtime()?;
            app.manage(RuntimeHost {
                runtime: Arc::new(runtime),
                shell: WindowsShellBackend,
                startup_launch: startup_launch_for_setup.clone(),
            });

            let app_handle = app.app_handle();
            let _ = emit_status(&app_handle, "Ruststrap_tauri_backend_ready");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
