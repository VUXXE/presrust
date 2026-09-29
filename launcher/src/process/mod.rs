pub mod mariadb;
pub mod nginx;
pub mod php;

use crate::config::{AppConfig, EnvPaths};
use anyhow::{bail, Result};
use mariadb::MariaDbService;
use nginx::NginxService;
use php::PhpService;
use std::net::TcpListener;

pub struct ProcessManager {
    pub mariadb: MariaDbService,
    pub php: PhpService,
    pub nginx: NginxService,
}

impl ProcessManager {
    pub fn new(config: &AppConfig) -> Self {
        Self {
            mariadb: MariaDbService::new(config.db_port),
            php: PhpService::new(config.php_port),
            nginx: NginxService::new(config.web_port, config.php_port),
        }
    }

    pub fn is_port_available(port: u16) -> bool {
        TcpListener::bind(("127.0.0.1", port)).is_ok()
    }

    pub fn is_running(&mut self) -> bool {
        self.mariadb.is_running() || self.php.is_running() || self.nginx.is_running()
    }

    pub fn cleanup_orphaned_processes(paths: &EnvPaths) {
        let pids = [
            paths.tmp_dir.join("mariadb.pid"),
            paths.tmp_dir.join("php.pid"),
            paths.tmp_dir.join("nginx.pid"),
        ];

        let mut killed = false;
        for pid_file in &pids {
            if let Ok(content) = std::fs::read_to_string(pid_file) {
                if let Ok(pid) = content.trim().parse::<i32>() {
                    #[cfg(unix)]
                    unsafe {
                        if libc::kill(pid as libc::pid_t, 0) == 0 {
                            let _ = libc::kill(pid as libc::pid_t, libc::SIGTERM);
                            killed = true;
                        }
                    }
                    #[cfg(windows)]
                    {
                        let _ = std::process::Command::new("taskkill")
                            .args(["/F", "/T", "/PID", &pid.to_string()])
                            .status();
                        killed = true;
                    }
                }
                let _ = std::fs::remove_file(pid_file);
            }
        }

        if killed {
            std::thread::sleep(std::time::Duration::from_millis(500));
        }
    }

    pub fn start_all(&mut self, paths: &EnvPaths) -> Result<()> {
        Self::cleanup_orphaned_processes(paths);

        // Check port conflicts before starting
        if !self.mariadb.is_running() && !Self::is_port_available(self.mariadb.port) {
            bail!(
                "Port MariaDB ({}) sudah digunakan oleh aplikasi lain",
                self.mariadb.port
            );
        }
        if !self.php.is_running() && !Self::is_port_available(self.php.port) {
            bail!(
                "Port PHP ({}) sudah digunakan oleh aplikasi lain",
                self.php.port
            );
        }
        if !self.nginx.is_running() && !Self::is_port_available(self.nginx.port) {
            bail!(
                "Port Web ({}) sudah digunakan oleh aplikasi lain",
                self.nginx.port
            );
        }

        // 1. Start MariaDB
        self.mariadb.start(paths)?;

        // 2. Start PHP-CGI
        self.php.start(paths)?;

        // 3. Start Nginx
        self.nginx.start(paths)?;

        Ok(())
    }

    pub fn stop_all(&mut self) -> Result<()> {
        let _ = self.nginx.stop();
        let _ = self.php.stop();
        let _ = self.mariadb.stop();
        Ok(())
    }

    pub fn update_ports(&mut self, config: &AppConfig) {
        if !self.is_running() {
            self.mariadb.port = config.db_port;
            self.php.port = config.php_port;
            self.nginx.port = config.web_port;
            self.nginx.php_port = config.php_port;
        }
    }
}

impl Drop for ProcessManager {
    fn drop(&mut self) {
        let _ = self.stop_all();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_port_availability() {
        // High ephemeral port is typically available
        let available = ProcessManager::is_port_available(48921);
        assert!(available);
    }

    #[test]
    fn test_process_manager_creation() {
        let config = AppConfig::default();
        let mut pm = ProcessManager::new(&config);
        assert!(!pm.is_running());
    }

    #[test]
    fn test_services_stay_running_in_thread() {
        let paths = match EnvPaths::resolve() {
            Ok(p) => p,
            Err(_) => return,
        };
        // Skip if runtime binaries are not present (e.g. CI without bundle)
        if NginxService::find_binary(&paths).is_err()
            || PhpService::find_binary(&paths).is_err()
            || MariaDbService::find_binary(&paths).is_err()
        {
            eprintln!("Skipping test: binaries not present");
            return;
        }

        let config = AppConfig::default();
        let pm = std::sync::Arc::new(std::sync::Mutex::new(ProcessManager::new(&config)));

        // Spawn in worker thread exactly like main.rs
        let pm_clone = pm.clone();
        let paths_clone = paths.clone();
        let handle = std::thread::spawn(move || {
            let mut manager = pm_clone.lock().unwrap();
            manager
                .start_all(&paths_clone)
                .expect("Failed to start services");
        });
        handle.join().unwrap(); // worker thread exits here!

        // Wait 1.5 seconds (under old PR_SET_PDEATHSIG, services died immediately)
        std::thread::sleep(std::time::Duration::from_millis(1500));

        let mut manager = pm.lock().unwrap();
        let running = manager.is_running();
        assert!(
            running,
            "Services should remain running after spawn thread exits!"
        );

        // Query /install/ to verify Nginx -> PHP-FPM -> PrestaShop installer executes
        if let Ok(mut stream) = std::net::TcpStream::connect("127.0.0.1:8080") {
            use std::io::{Read, Write};
            let _ = stream.write_all(
                b"GET /install/ HTTP/1.1\r\nHost: 127.0.0.1:8080\r\nConnection: close\r\n\r\n",
            );
            let mut response = String::new();
            let _ = stream.read_to_string(&mut response);
            println!(
                "HTTP Response preview:\n{}",
                response.lines().take(15).collect::<Vec<_>>().join("\n")
            );
            assert!(
                !response.contains("We can&#039;t start installation") && !response.contains("We can't start installation"),
                "PrestaShop installer should not fail with missing requirements! Response snippet: {}",
                &response[..response.len().min(500)]
            );
        }

        // Query /install/index.php?step=system to verify System compatibility step succeeds
        if let Ok(mut stream) = std::net::TcpStream::connect("127.0.0.1:8080") {
            use std::io::{Read, Write};
            let _ = stream.write_all(
                b"GET /install/index.php?step=system HTTP/1.1\r\nHost: 127.0.0.1:8080\r\nConnection: close\r\n\r\n",
            );
            let mut response = String::new();
            let _ = stream.read_to_string(&mut response);
            println!(
                "System step preview:\n{}",
                response.lines().take(15).collect::<Vec<_>>().join("\n")
            );
            assert!(
                !response.contains("500 Internal Server Error"),
                "System compatibility step must not return 500 error! Response snippet: {}",
                &response[..response.len().min(500)]
            );
        }

        // Verify required theme assets for installation exist
        let app_dir = &paths.app_dir;
        if app_dir.exists() {
            let classic_theme = app_dir.join("themes/classic");
            if classic_theme.exists() {
                assert!(
                    classic_theme.join("assets/js/theme.js").exists(),
                    "classic theme must have assets/js/theme.js"
                );
                assert!(
                    classic_theme.join("assets/css/theme.css").exists(),
                    "classic theme must have assets/css/theme.css"
                );
                assert!(
                    classic_theme.join("config/theme.yml").exists(),
                    "classic theme must have config/theme.yml"
                );
            }
        }

        manager.stop_all().expect("Failed to stop services");
    }
}
