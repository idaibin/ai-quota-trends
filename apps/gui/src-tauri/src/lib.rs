mod commands;
mod state;
mod update;

use std::{
    fs, io,
    path::Path,
    sync::{Arc, Mutex},
};

use ai_quota_core::{CollectorConfig, CollectorRuntime, Database, TokenUsageRuntime};
use state::AppState;
use tauri::{
    Manager, PhysicalPosition, RunEvent, WindowEvent,
    image::Image,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};
use tauri_plugin_autostart::ManagerExt;

const TRAY_REFRESH_INTERVAL: std::time::Duration = std::time::Duration::from_secs(30);
const LEGACY_APP_IDENTIFIER: &str = "dev.idaibin.codex-quota-trends";
const APP_IDENTIFIER: &str = "dev.idaibin.ai-quota-trends";
const LEGACY_AUTOSTART_NAMES: [&str; 2] = ["Agent Quota Trends.plist", "Codex Quota Trends.plist"];

fn migrate_legacy_app_data(data_dir: &Path) -> io::Result<bool> {
    if data_dir.exists() {
        return Ok(false);
    }
    let Some(parent) = data_dir.parent() else { return Ok(false) };
    let legacy_data_dir = parent.join(LEGACY_APP_IDENTIFIER);
    if !legacy_data_dir.exists() {
        return Ok(false);
    }
    fs::rename(legacy_data_dir, data_dir)?;
    Ok(true)
}

fn migrate_legacy_preferences(data_dir: &Path) -> io::Result<bool> {
    let Some(library_dir) = data_dir.parent().and_then(Path::parent) else { return Ok(false) };
    let preferences_dir = library_dir.join("Preferences");
    let legacy_preferences = preferences_dir.join(format!("{LEGACY_APP_IDENTIFIER}.plist"));
    let current_preferences = preferences_dir.join(format!("{APP_IDENTIFIER}.plist"));
    if current_preferences.exists() || !legacy_preferences.exists() {
        return Ok(false);
    }
    fs::rename(legacy_preferences, current_preferences)?;
    Ok(true)
}

fn remove_legacy_autostart_entries(data_dir: &Path) -> io::Result<()> {
    let Some(library_dir) = data_dir.parent().and_then(Path::parent) else { return Ok(()) };
    let launch_agents_dir = library_dir.join("LaunchAgents");
    for name in LEGACY_AUTOSTART_NAMES {
        let path = launch_agents_dir.join(name);
        if path.exists() {
            fs::remove_file(path)?;
        }
    }
    Ok(())
}

fn format_remaining_title(used_percent: Option<f64>) -> String {
    used_percent
        .filter(|value| value.is_finite())
        .map(|value| format!("{:.0}%", 100.0 - value.clamp(0.0, 100.0)))
        .unwrap_or_else(|| "--%".to_owned())
}

fn current_remaining_title(database: &Arc<Mutex<Database>>) -> String {
    let used_percent = database.lock().ok().and_then(|database| {
        database
            .latest_any_snapshot()
            .ok()
            .flatten()
            .and_then(|snapshot| snapshot.windows.first().map(|window| window.used_percent))
    });
    format_remaining_title(used_percent)
}

fn show_main(app: &tauri::AppHandle, route: Option<&str>) {
    if let Some(window) = app.get_webview_window("main") {
        if route == Some("settings") {
            let _ = window.eval(
                "window.dispatchEvent(new CustomEvent('aqt-route-requested', { detail: 'settings' }))",
            );
        }
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

#[tauri::command]
fn open_settings(app: tauri::AppHandle) {
    show_main(&app, Some("settings"));
}

fn toggle_tray(app: &tauri::AppHandle, anchor_x: f64, anchor_y: f64) {
    let Some(window) = app.get_webview_window("tray") else { return };
    if window.is_visible().unwrap_or(false) {
        let _ = window.hide();
        return;
    }
    let scale = window.scale_factor().unwrap_or(1.0);
    let width = window.outer_size().map(|size| size.width as f64).unwrap_or(420.0 * scale);
    let _ = window.set_position(PhysicalPosition::new((anchor_x - width / 2.0).max(8.0), anchor_y));
    let _ = window.show();
    let _ = window.set_focus();
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let updater = tauri_plugin_updater::Builder::new().target("darwin-universal");
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_autostart::Builder::new().app_name("AI Quota Trends").build())
        .plugin(updater.build())
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let data_dir = app.path().app_data_dir()?;
            migrate_legacy_preferences(&data_dir)?;
            migrate_legacy_app_data(&data_dir)?;
            fs::create_dir_all(&data_dir)?;
            let database = Database::open(data_dir.join("quota-trends.db"))?;
            let launch_at_login = database.load_settings()?.launch_at_login;
            let database = Arc::new(Mutex::new(database));
            let autolaunch = app.autolaunch();
            if launch_at_login {
                autolaunch.enable()?;
            } else {
                autolaunch.disable()?;
            }
            remove_legacy_autostart_entries(&data_dir)?;
            let tray_database = Arc::clone(&database);
            let initial_tray_title = current_remaining_title(&tray_database);
            let runtime = CollectorRuntime::new(Arc::clone(&database), CollectorConfig::default());
            let token_runtime = TokenUsageRuntime::from_environment(Arc::clone(&database))?;
            let state = AppState {
                database,
                collector_state: runtime.state(),
                collector_refresh: runtime.refresh_notifier(),
                collector_reload: runtime.reload_notifier(),
                data_dir,
            };
            app.manage(state);
            tauri::async_runtime::spawn(runtime.run());
            tauri::async_runtime::spawn(token_runtime.run());

            let settings = MenuItem::with_id(app, "settings", "设置", true, Some("CmdOrCtrl+,"))?;
            let separator = PredefinedMenuItem::separator(app)?;
            let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&settings, &separator, &quit])?;
            let tray_icon = TrayIconBuilder::with_id("ai-quota-trends-tray")
                .icon(Image::new(include_bytes!("../icons/tray-template.rgba"), 128, 128))
                .icon_as_template(true)
                .title(&initial_tray_title)
                .tooltip("AI Quota Trends")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        rect,
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let position = rect.position.to_physical::<f64>(1.0);
                        let size = rect.size.to_physical::<f64>(1.0);
                        toggle_tray(
                            tray.app_handle(),
                            position.x + size.width / 2.0,
                            position.y + size.height,
                        );
                    }
                })
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "settings" => show_main(app, Some("settings")),
                    "quit" => app.exit(0),
                    _ => {}
                })
                .build(app)?;
            tauri::async_runtime::spawn(async move {
                let mut displayed_title = initial_tray_title;
                loop {
                    tokio::time::sleep(TRAY_REFRESH_INTERVAL).await;
                    let next_title = current_remaining_title(&tray_database);
                    if next_title != displayed_title {
                        let _ = tray_icon.set_title(Some(&next_title));
                        displayed_title = next_title;
                    }
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_dashboard,
            commands::refresh_quota,
            commands::get_activity,
            commands::get_alerts,
            commands::get_settings,
            commands::list_providers,
            commands::list_provider_quotas,
            commands::save_settings,
            commands::export_data,
            commands::open_data_folder,
            commands::get_database_stats,
            commands::cleanup_database,
            commands::reset_local_data,
            open_settings,
            update::get_app_version,
            update::check_for_update,
            update::install_update,
            update::restart_app,
        ])
        .build(tauri::generate_context!())
        .expect("failed to build AI Quota Trends");

    app.run(|handle, event| match event {
        RunEvent::WindowEvent { label, event: WindowEvent::CloseRequested { api, .. }, .. }
            if label == "main" =>
        {
            api.prevent_close();
            if let Some(window) = handle.get_webview_window("main") {
                let _ = window.hide();
            }
        }
        RunEvent::WindowEvent { label, event: WindowEvent::Focused(false), .. }
            if label == "tray" =>
        {
            if let Some(window) = handle.get_webview_window("tray") {
                let _ = window.hide();
            }
        }
        _ => {}
    });
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::{
        format_remaining_title, migrate_legacy_app_data, migrate_legacy_preferences,
        remove_legacy_autostart_entries,
    };

    #[test]
    fn formats_remaining_percentage_for_the_tray_title() {
        assert_eq!(format_remaining_title(Some(42.4)), "58%");
        assert_eq!(format_remaining_title(Some(-3.0)), "100%");
        assert_eq!(format_remaining_title(Some(120.0)), "0%");
        assert_eq!(format_remaining_title(None), "--%");
        assert_eq!(format_remaining_title(Some(f64::NAN)), "--%");
    }

    #[test]
    fn migrates_the_legacy_app_data_directory_before_opening_the_database() {
        let parent = tempfile::tempdir().unwrap();
        let legacy = parent.path().join("dev.idaibin.codex-quota-trends");
        let current = parent.path().join("dev.idaibin.ai-quota-trends");
        fs::create_dir_all(legacy.join("exports")).unwrap();
        fs::write(legacy.join("quota-trends.db"), b"existing database").unwrap();
        fs::write(legacy.join("quota-trends.db-wal"), b"existing wal").unwrap();
        fs::write(legacy.join("exports/history.csv"), b"existing export").unwrap();

        assert!(migrate_legacy_app_data(&current).unwrap());

        assert!(!legacy.exists());
        assert_eq!(fs::read(current.join("quota-trends.db")).unwrap(), b"existing database");
        assert_eq!(fs::read(current.join("quota-trends.db-wal")).unwrap(), b"existing wal");
        assert_eq!(fs::read(current.join("exports/history.csv")).unwrap(), b"existing export");
    }

    #[test]
    fn never_overwrites_an_existing_current_app_data_directory() {
        let parent = tempfile::tempdir().unwrap();
        let legacy = parent.path().join("dev.idaibin.codex-quota-trends");
        let current = parent.path().join("dev.idaibin.ai-quota-trends");
        fs::create_dir_all(&legacy).unwrap();
        fs::create_dir_all(&current).unwrap();
        fs::write(legacy.join("quota-trends.db"), b"legacy").unwrap();
        fs::write(current.join("quota-trends.db"), b"current").unwrap();

        assert!(!migrate_legacy_app_data(&current).unwrap());

        assert_eq!(fs::read(legacy.join("quota-trends.db")).unwrap(), b"legacy");
        assert_eq!(fs::read(current.join("quota-trends.db")).unwrap(), b"current");
    }

    #[test]
    fn removes_only_the_two_legacy_product_autostart_entries() {
        let library = tempfile::tempdir().unwrap();
        let data_dir = library.path().join("Application Support/dev.idaibin.ai-quota-trends");
        let launch_agents = library.path().join("LaunchAgents");
        fs::create_dir_all(&data_dir).unwrap();
        fs::create_dir_all(&launch_agents).unwrap();
        fs::write(launch_agents.join("Agent Quota Trends.plist"), b"legacy").unwrap();
        fs::write(launch_agents.join("Codex Quota Trends.plist"), b"legacy").unwrap();
        fs::write(launch_agents.join("Unrelated.plist"), b"keep").unwrap();

        remove_legacy_autostart_entries(&data_dir).unwrap();

        assert!(!launch_agents.join("Agent Quota Trends.plist").exists());
        assert!(!launch_agents.join("Codex Quota Trends.plist").exists());
        assert_eq!(fs::read(launch_agents.join("Unrelated.plist")).unwrap(), b"keep");
    }

    #[test]
    fn migrates_legacy_macos_preferences_without_overwriting_current_preferences() {
        let library = tempfile::tempdir().unwrap();
        let data_dir = library.path().join("Application Support/dev.idaibin.ai-quota-trends");
        let preferences = library.path().join("Preferences");
        fs::create_dir_all(&preferences).unwrap();
        let legacy = preferences.join("dev.idaibin.codex-quota-trends.plist");
        let current = preferences.join("dev.idaibin.ai-quota-trends.plist");
        fs::write(&legacy, b"preferred menu position").unwrap();

        assert!(migrate_legacy_preferences(&data_dir).unwrap());
        assert!(!legacy.exists());
        assert_eq!(fs::read(&current).unwrap(), b"preferred menu position");

        fs::write(&legacy, b"stale preferences").unwrap();
        assert!(!migrate_legacy_preferences(&data_dir).unwrap());
        assert_eq!(fs::read(&legacy).unwrap(), b"stale preferences");
        assert_eq!(fs::read(&current).unwrap(), b"preferred menu position");
    }
}
