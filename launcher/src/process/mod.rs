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

    pub fn start_all(&mut self, paths: &EnvPaths) -> Result<()> {
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
}
