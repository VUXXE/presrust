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
    }
}

#[tauri::command]
fn start_services(state: State<'_, AppRuntime>) -> Result<(), String> {
    {
        let mut busy = state.is_busy.lock().unwrap();
        *busy = true;
    }

    let pm = state.pm.clone();
    let paths = state.paths.clone();
    let busy_clone = state.is_busy.clone();

    std::thread::spawn(move || {
        let start_res = {
            let mut manager = pm.lock().unwrap();
            manager.start_all(&paths)
        };
        let mut busy = busy_clone.lock().unwrap();
        *busy = false;
        if let Err(e) = start_res {
            eprintln!("Failed to start services: {}", e);
        }
    });

    Ok(())
}

#[tauri::command]
fn stop_services(state: State<'_, AppRuntime>) -> Result<(), String> {
    {
        let mut busy = state.is_busy.lock().unwrap();
        *busy = true;
    }

    let pm = state.pm.clone();
    let busy_clone = state.is_busy.clone();

    std::thread::spawn(move || {
        let stop_res = {
            let mut manager = pm.lock().unwrap();
            manager.stop_all()
        };
        let mut busy = busy_clone.lock().unwrap();
        *busy = false;
        if let Err(e) = stop_res {
            eprintln!("Failed to stop services: {}", e);
        }
    });

    Ok(())
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
            save_settings
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
