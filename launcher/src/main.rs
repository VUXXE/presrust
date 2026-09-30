#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;
mod monitor;
mod process;

use config::{AppConfig, EnvPaths};
use monitor::LogMonitor;
use process::ProcessManager;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tauri::{Emitter, Manager, State};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AppStateDto {
    pub is_running: bool,
    pub is_setup_mode: bool,
    pub is_busy: bool,
    pub host_url: String,
    pub admin_folder: Option<String>,
    pub db_host: String,
    pub db_port: u16,
    pub db_user: String,
    pub db_name: String,
    pub web_port: u16,
    pub php_port: u16,
    pub app_version: String,
}

pub struct AppRuntime {
    pub paths: Arc<EnvPaths>,
    pub config: Arc<Mutex<AppConfig>>,
    pub pm: Arc<Mutex<ProcessManager>>,
    pub log_history: Arc<Mutex<Vec<String>>>,
    pub is_busy: Arc<Mutex<bool>>,
}

#[tauri::command]
fn get_app_state(state: State<'_, AppRuntime>) -> AppStateDto {
    let cfg = state.config.lock().unwrap();
    let is_running = state.pm.lock().unwrap().is_running();
    let is_busy = *state.is_busy.lock().unwrap();
    let (is_setup, admin) = state.paths.detect_prestashop_state();

    AppStateDto {
        is_running,
        is_setup_mode: is_setup,
        is_busy,
        host_url: format!("http://127.0.0.1:{}", cfg.web_port),
        admin_folder: admin,
        db_host: "127.0.0.1".into(),
        db_port: cfg.db_port,
        db_user: "root".into(),
        db_name: "prestashop".into(),
        web_port: cfg.web_port,
        php_port: cfg.php_port,
        app_version: env!("CARGO_PKG_VERSION").to_string(),
    }
}

#[tauri::command]
fn start_services(state: State<'_, AppRuntime>) -> Result<(), String> {
    {
        let mut busy = state.is_busy.lock().unwrap();
        *busy = true;
    }

    state.paths.ensure_runtime_permissions();

    let res = {
        let mut manager = state.pm.lock().unwrap();
        manager.start_all(&state.paths)
    };

    {
        let mut busy = state.is_busy.lock().unwrap();
        *busy = false;
    }

    res.map_err(|e| {
        eprintln!("Failed to start services: {:#}", e);
        format!("{:#}", e)
    })
}

#[tauri::command]
fn stop_services(state: State<'_, AppRuntime>) -> Result<(), String> {
    {
        let mut busy = state.is_busy.lock().unwrap();
        *busy = true;
    }

    let res = {
        let mut manager = state.pm.lock().unwrap();
        manager.stop_all()
    };

    {
        let mut busy = state.is_busy.lock().unwrap();
        *busy = false;
    }

    res.map_err(|e| {
        eprintln!("Failed to stop services: {:#}", e);
        format!("{:#}", e)
    })
}

#[tauri::command]
fn open_shop(state: State<'_, AppRuntime>) -> Result<(), String> {
    let (is_setup, _) = state.paths.detect_prestashop_state();
    let cfg = state.config.lock().unwrap();
    let url = if is_setup {
        format!("http://127.0.0.1:{}/install/", cfg.web_port)
    } else {
        format!("http://127.0.0.1:{}/", cfg.web_port)
    };
    let _ = open::that(&url);
    Ok(())
}

#[tauri::command]
fn open_admin(state: State<'_, AppRuntime>) -> Result<(), String> {
    let (_, admin_folder) = state.paths.detect_prestashop_state();
    if let Some(admin) = admin_folder {
        let cfg = state.config.lock().unwrap();
        let url = format!("http://127.0.0.1:{}/{}/", cfg.web_port, admin);
        let _ = open::that(&url);
    }
    Ok(())
}

#[tauri::command]
fn open_logs_folder(state: State<'_, AppRuntime>) -> Result<(), String> {
    let _ = open::that(&state.paths.logs_dir);
    Ok(())
}

#[tauri::command]
fn get_logs(state: State<'_, AppRuntime>) -> Vec<String> {
    state.log_history.lock().unwrap().clone()
}

#[tauri::command]
fn clear_logs(state: State<'_, AppRuntime>) -> Result<(), String> {
    state.log_history.lock().unwrap().clear();
    Ok(())
}

#[tauri::command]
fn save_settings(
    state: State<'_, AppRuntime>,
    web_port: String,
    php_port: String,
    db_port: String,
) -> Result<(), String> {
    let mut cfg = state.config.lock().unwrap();
    if let Ok(w) = web_port.parse::<u16>() {
        cfg.web_port = w;
    }
    if let Ok(p) = php_port.parse::<u16>() {
        cfg.php_port = p;
    }
    if let Ok(d) = db_port.parse::<u16>() {
        cfg.db_port = d;
    }
    state.pm.lock().unwrap().update_ports(&cfg);
    Ok(())
}

#[tauri::command]
fn reinstall_prestashop(state: State<'_, AppRuntime>) -> Result<(), String> {
    {
        let mut busy = state.is_busy.lock().unwrap();
        *busy = true;
    }

    let res = (|| -> anyhow::Result<()> {
        let was_running = {
            let mut manager = state.pm.lock().unwrap();
            let running = manager.is_running();
            if running {
                let _ = manager.stop_all();
            }
            running
        };

        std::thread::sleep(std::time::Duration::from_millis(600));

        // 1. Clean temporary lock/pid/session files
        let tmp_dir = &state.paths.tmp_dir;
        let _ = std::fs::remove_file(tmp_dir.join("mysql.sock"));
        let _ = std::fs::remove_file(tmp_dir.join("mariadb.pid"));
        let _ = std::fs::remove_file(tmp_dir.join("nginx.pid"));
        let _ = std::fs::remove_file(tmp_dir.join("php.pid"));

        let sessions = tmp_dir.join("sessions");
        if sessions.exists() {
            let _ = std::fs::remove_dir_all(&sessions);
            let _ = std::fs::create_dir_all(&sessions);
        }

        let uploads = tmp_dir.join("uploads");
        if uploads.exists() {
            let _ = std::fs::remove_dir_all(&uploads);
            let _ = std::fs::create_dir_all(&uploads);
        }

        // 2. Reset database data
        let db_dir = state.paths.data_dir.join("mariadb/prestashop");
        if db_dir.exists() {
            let _ = std::fs::remove_dir_all(&db_dir);
        }

        // 3. Reset PrestaShop generated files, caches, and parameters
        let app_dir = &state.paths.app_dir;
        if app_dir.exists() {
            let param_candidates = [
                app_dir.join("app/config/parameters.php"),
                app_dir.join("config/parameters.php"),
                app_dir.join("app/config/parameters.yml"),
            ];
            for p in &param_candidates {
                if p.exists() {
                    let _ = std::fs::remove_file(p);
                }
            }

            let cache_dir = app_dir.join("var/cache");
            if cache_dir.exists() {
                let _ = std::fs::remove_dir_all(&cache_dir);
                let _ = std::fs::create_dir_all(&cache_dir);
            }

            let var_logs = app_dir.join("var/logs");
            if var_logs.exists() {
                let _ = std::fs::remove_dir_all(&var_logs);
                let _ = std::fs::create_dir_all(&var_logs);
            }

            // Restore /install if renamed (e.g. install_bak, install.bak)
            let install_dir = app_dir.join("install");
            if !install_dir.exists() {
                for backup_name in &["install_bak", "install.bak", "install_old", "install.old"] {
                    let backup_path = app_dir.join(backup_name);
                    if backup_path.exists() {
                        let _ = std::fs::rename(&backup_path, &install_dir);
                        break;
                    }
                }
            }

            // Restore /admin if renamed
            if let Ok(entries) = std::fs::read_dir(app_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                            if name.starts_with("admin")
                                && name != "admin"
                                && name != "admin-api"
                                && name != "admin-dev"
                            {
                                let admin_dir = app_dir.join("admin");
                                if !admin_dir.exists() {
                                    let _ = std::fs::rename(&path, &admin_dir);
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }

        // 4. Ensure permissions
        state.paths.ensure_runtime_permissions();

        // 5. Restart services if they were running
        if was_running {
            let mut manager = state.pm.lock().unwrap();
            manager.start_all(&state.paths)?;
        }

        Ok(())
    })();

    {
        let mut busy = state.is_busy.lock().unwrap();
        *busy = false;
    }

    res.map_err(|e| e.to_string())
}

fn main() {
    let paths = Arc::new(EnvPaths::resolve().unwrap_or_else(|err| {
        eprintln!("Fatal: cannot resolve runtime paths: {}", err);
        panic!("Fatal: cannot resolve runtime paths");
    }));

    let config = Arc::new(Mutex::new(AppConfig::default()));
    let pm = Arc::new(Mutex::new(ProcessManager::new(&config.lock().unwrap())));
    let log_history = Arc::new(Mutex::new(Vec::new()));
    let is_busy = Arc::new(Mutex::new(false));

    // Start Log Monitor thread
    let log_monitor = LogMonitor::start(paths.logs_dir.clone());
    let log_history_clone = log_history.clone();

    let runtime = AppRuntime {
        paths: paths.clone(),
        config: config.clone(),
        pm: pm.clone(),
        log_history: log_history.clone(),
        is_busy: is_busy.clone(),
    };

    tauri::Builder::default()
        .manage(runtime)
        .invoke_handler(tauri::generate_handler![
            get_app_state,
            start_services,
            stop_services,
            open_shop,
            open_admin,
            open_logs_folder,
            get_logs,
            clear_logs,
            save_settings,
            reinstall_prestashop
        ])
        .setup(move |app| {
            let app_handle = app.handle().clone();
            std::thread::spawn(move || {
                while let Ok(entry) = log_monitor.receiver.recv() {
                    let formatted = entry.formatted_line.clone();
                    {
                        let mut hist = log_history_clone.lock().unwrap();
                        hist.push(formatted.clone());
                        if hist.len() > 1000 {
                            hist.remove(0);
                        }
                    }
                    let _ = app_handle.emit("log-entry", formatted);
                }
            });
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                let state: State<AppRuntime> = window.state();
                let mut manager = state.pm.lock().unwrap();
                let _ = manager.stop_all();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
