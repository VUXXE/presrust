mod config;
mod monitor;
mod process;

use config::{AppConfig, EnvPaths};
use monitor::{LogEntry, LogMonitor};
use process::ProcessManager;
use slint::{ComponentHandle, SharedString, Timer, TimerMode};
use std::sync::{Arc, Mutex};
use std::time::Duration;

slint::include_modules!();

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let main_window = MainWindow::new()?;

    // 1. Resolve environment paths
    let paths = Arc::new(EnvPaths::resolve().unwrap_or_else(|err| {
        eprintln!("Warning: Error resolving paths: {}", err);
        panic!("Fatal: cannot resolve runtime paths");
    }));

    // 2. Load Configuration
    let config = Arc::new(Mutex::new(AppConfig::default()));

    // 3. Process Manager
    let pm = Arc::new(Mutex::new(ProcessManager::new(&config.lock().unwrap())));

    // 4. Log Monitor & History Buffer
    let log_monitor = LogMonitor::start(paths.logs_dir.clone());
    let log_history = Arc::new(Mutex::new(Vec::<LogEntry>::new()));
    let active_filter = Arc::new(Mutex::new(String::from("ALL")));

    // Initial PrestaShop state
    let (is_setup, admin_folder) = paths.detect_prestashop_state();

    // Set initial UI properties
    {
        let cfg = config.lock().unwrap();
        main_window.set_cfg_web_port(SharedString::from(cfg.web_port.to_string()));
        main_window.set_cfg_php_port(SharedString::from(cfg.php_port.to_string()));
        main_window.set_cfg_db_port(SharedString::from(cfg.db_port.to_string()));
        main_window.set_host_url(SharedString::from(format!(
            "http://127.0.0.1:{}",
            cfg.web_port
        )));
        main_window.set_db_host(SharedString::from("127.0.0.1"));
        main_window.set_db_port(SharedString::from(cfg.db_port.to_string()));
        main_window.set_db_user(SharedString::from("root"));
        main_window.set_db_name(SharedString::from("prestashop"));
        main_window.set_is_setup_mode(is_setup);
        if let Some(ref admin) = admin_folder {
            main_window.set_admin_folder(SharedString::from(admin.clone()));
        }
    }

    // Callback: Start Services
    {
        let pm = pm.clone();
        let paths = paths.clone();
        let ui_handle = main_window.as_weak();

        main_window.on_start_services(move || {
            let Some(ui) = ui_handle.upgrade() else {
                return;
            };
            ui.set_is_busy(true);

            let mut manager = pm.lock().unwrap();
            match manager.start_all(&paths) {
                Ok(_) => {
                    ui.set_is_running(true);
                    ui.set_status_text(SharedString::from("Berjalan"));
                    let (is_setup, admin) = paths.detect_prestashop_state();
                    ui.set_is_setup_mode(is_setup);
                    if let Some(admin_name) = admin {
                        ui.set_admin_folder(SharedString::from(admin_name));
                    }
                }
                Err(err) => {
                    let err_msg = format!("Gagal: {}", err);
                    eprintln!("{}", err_msg);
                    ui.set_status_text(SharedString::from(err_msg));
                    ui.set_is_running(false);
                }
            }
            ui.set_is_busy(false);
        });
    }

    // Callback: Stop Services
    {
        let pm = pm.clone();
        let ui_handle = main_window.as_weak();

        main_window.on_stop_services(move || {
            let Some(ui) = ui_handle.upgrade() else {
                return;
            };
            ui.set_is_busy(true);

            let mut manager = pm.lock().unwrap();
            let _ = manager.stop_all();
            ui.set_is_running(false);
            ui.set_status_text(SharedString::from("Berhenti"));
            ui.set_is_busy(false);
        });
    }

    // Callback: Open Shop
    {
        let config = config.clone();
        let paths = paths.clone();
        main_window.on_open_shop(move || {
            let (is_setup, _) = paths.detect_prestashop_state();
            let cfg = config.lock().unwrap();
            let url = if is_setup {
                format!("http://127.0.0.1:{}/install/", cfg.web_port)
            } else {
                format!("http://127.0.0.1:{}/", cfg.web_port)
            };
            let _ = open::that(&url);
        });
    }

    // Callback: Open Admin
    {
        let config = config.clone();
        let paths = paths.clone();
        main_window.on_open_admin(move || {
            let (_, admin_folder) = paths.detect_prestashop_state();
            if let Some(admin) = admin_folder {
                let cfg = config.lock().unwrap();
                let url = format!("http://127.0.0.1:{}/{}/", cfg.web_port, admin);
                let _ = open::that(&url);
            }
        });
    }

    // Callback: Copy DB Info
    {
        let config = config.clone();
        let ui_handle = main_window.as_weak();

        main_window.on_copy_db_info(move || {
            let cfg = config.lock().unwrap();
            let text = format!(
                "Host: 127.0.0.1\nPort: {}\nUser: root\nPassword: \nDatabase: prestashop",
                cfg.db_port
            );
            if let Ok(mut clipboard) = arboard::Clipboard::new() {
                let _ = clipboard.set_text(text);
                if let Some(ui) = ui_handle.upgrade() {
                    ui.set_db_copy_text(SharedString::from("Tersalin!"));
                }
            }
        });
    }

    // Callback: Copy Logs
    {
        let ui_handle = main_window.as_weak();
        let log_history = log_history.clone();
        let active_filter = active_filter.clone();

        main_window.on_copy_logs(move || {
            let filter = active_filter.lock().unwrap().clone();
            let logs = log_history.lock().unwrap();
            let filtered_text: Vec<String> = logs
                .iter()
                .filter(|entry| filter == "ALL" || entry.source == filter)
                .map(|e| e.formatted_line.clone())
                .collect();
            let all_logs = filtered_text.join("\n");

            if let Ok(mut clipboard) = arboard::Clipboard::new() {
                let _ = clipboard.set_text(all_logs);
                if let Some(ui) = ui_handle.upgrade() {
                    ui.set_copy_log_text(SharedString::from("Tersalin!"));
                }
            }
        });
    }

    // Callback: Clear Logs
    {
        let log_history = log_history.clone();
        let ui_handle = main_window.as_weak();

        main_window.on_clear_logs(move || {
            log_history.lock().unwrap().clear();
            if let Some(ui) = ui_handle.upgrade() {
                ui.set_log_content(SharedString::from(""));
            }
        });
    }

    // Callback: Open Logs Folder
    {
        let paths = paths.clone();
        main_window.on_open_logs_folder(move || {
            let _ = open::that(&paths.logs_dir);
        });
    }

    // Callback: Toggle Log Filter
    {
        let active_filter = active_filter.clone();
        let log_history = log_history.clone();
        let ui_handle = main_window.as_weak();

        main_window.on_toggle_log_filter(move |filter| {
            let filter_str = filter.to_string();
            *active_filter.lock().unwrap() = filter_str.clone();

            if let Some(ui) = ui_handle.upgrade() {
                ui.set_log_filter(SharedString::from(filter_str.clone()));
                let logs = log_history.lock().unwrap();
                let filtered: Vec<String> = logs
                    .iter()
                    .filter(|e| filter_str == "ALL" || e.source == filter_str)
                    .map(|e| e.formatted_line.clone())
                    .collect();
                ui.set_log_content(SharedString::from(filtered.join("\n")));
            }
        });
    }

    // Callback: Save Settings
    {
        let config = config.clone();
        let pm = pm.clone();
        let ui_handle = main_window.as_weak();

        main_window.on_save_settings(move |web_str, php_str, db_str| {
            let mut cfg = config.lock().unwrap();
            if let Ok(p) = web_str.parse::<u16>() {
                cfg.web_port = p;
            }
            if let Ok(p) = php_str.parse::<u16>() {
                cfg.php_port = p;
            }
            if let Ok(p) = db_str.parse::<u16>() {
                cfg.db_port = p;
            }

            pm.lock().unwrap().update_ports(&cfg);

            if let Some(ui) = ui_handle.upgrade() {
                ui.set_cfg_web_port(SharedString::from(cfg.web_port.to_string()));
                ui.set_cfg_php_port(SharedString::from(cfg.php_port.to_string()));
                ui.set_cfg_db_port(SharedString::from(cfg.db_port.to_string()));
                ui.set_db_port(SharedString::from(cfg.db_port.to_string()));
                ui.set_host_url(SharedString::from(format!(
                    "http://127.0.0.1:{}",
                    cfg.web_port
                )));
            }
        });
    }

    // Background Timer (every 400ms): Poll logs & state updates
    let timer = Timer::default();
    {
        let ui_handle = main_window.as_weak();
        let paths = paths.clone();
        let log_history = log_history.clone();
        let active_filter = active_filter.clone();
        let pm = pm.clone();

        timer.start(TimerMode::Repeated, Duration::from_millis(400), move || {
            let Some(ui) = ui_handle.upgrade() else {
                return;
            };

            // 1. Drain new log entries
            let mut new_entries = Vec::new();
            while let Ok(entry) = log_monitor.receiver.try_recv() {
                new_entries.push(entry);
            }

            if !new_entries.is_empty() {
                let mut history = log_history.lock().unwrap();
                history.extend(new_entries);
                // Keep last 1500 lines to avoid high memory
                if history.len() > 1500 {
                    let excess = history.len() - 1500;
                    history.drain(0..excess);
                }

                let filter = active_filter.lock().unwrap().clone();
                let filtered: Vec<String> = history
                    .iter()
                    .filter(|e| filter == "ALL" || e.source == filter)
                    .map(|e| e.formatted_line.clone())
                    .collect();
                ui.set_log_content(SharedString::from(filtered.join("\n")));
            }

            // 2. Check process health if supposed to be running
            if ui.get_is_running() {
                let mut manager = pm.lock().unwrap();
                if !manager.is_running() {
                    ui.set_is_running(false);
                    ui.set_status_text(SharedString::from("Layanan terhenti"));
                }
            }

            // 3. Dynamic admin folder check
            let (is_setup, admin) = paths.detect_prestashop_state();
            if ui.get_is_setup_mode() != is_setup {
                ui.set_is_setup_mode(is_setup);
            }
            if let Some(admin_name) = admin {
                if ui.get_admin_folder() != admin_name {
                    ui.set_admin_folder(SharedString::from(admin_name));
                }
            }
        });
    }

    // Run Slint Event Loop
    main_window.run()?;

    // On window close, ensure all processes are stopped
    let mut manager = pm.lock().unwrap();
    let _ = manager.stop_all();

    Ok(())
}
