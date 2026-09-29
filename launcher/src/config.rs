use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub web_port: u16,
    pub php_port: u16,
    pub db_port: u16,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            web_port: 8080,
            php_port: 9000,
            db_port: 3306,
        }
    }
}

pub struct EnvPaths {
    pub root_dir: PathBuf,
    pub app_dir: PathBuf,
    pub runtime_dir: PathBuf,
    pub config_dir: PathBuf,
    pub logs_dir: PathBuf,
    pub data_dir: PathBuf,
    pub tmp_dir: PathBuf,
}

impl EnvPaths {
    pub fn resolve() -> Result<Self> {
        let mut candidates = Vec::new();
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(parent) = exe_path.parent() {
                candidates.push(parent.to_path_buf());
            }
        }
        if let Ok(cwd) = std::env::current_dir() {
            candidates.push(cwd);
        }

        let mut root_dir = None;
        for start in candidates {
            let mut curr = start;
            for _ in 0..5 {
                if curr.join("config").exists()
                    || curr.join("versions.env").exists()
                    || curr.join("prd.md").exists()
                {
                    root_dir = Some(curr);
                    break;
                }
                if let Some(parent) = curr.parent() {
                    curr = parent.to_path_buf();
                } else {
                    break;
                }
            }
            if root_dir.is_some() {
                break;
            }
        }

        let root_dir = root_dir.unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
        let mut app_dir = root_dir.join("app");
        if !app_dir.exists() && root_dir.join("prestashop").exists() {
            app_dir = root_dir.join("prestashop");
        }

        let runtime_dir = root_dir.join("runtime");
        let config_dir = root_dir.join("config");
        let logs_dir = root_dir.join("logs");
        let data_dir = root_dir.join("data");
        let tmp_dir = root_dir.join("tmp");

        fs::create_dir_all(&logs_dir)?;
        fs::create_dir_all(tmp_dir.join("sessions"))?;
        fs::create_dir_all(tmp_dir.join("uploads"))?;
        fs::create_dir_all(data_dir.join("mariadb"))?;

        Ok(Self {
            root_dir,
            app_dir,
            runtime_dir,
            config_dir,
            logs_dir,
            data_dir,
            tmp_dir,
        })
    }

    /// Detect active PrestaShop state:
    /// Returns (is_setup_mode, detected_admin_folder_name)
    pub fn detect_prestashop_state(&self) -> (bool, Option<String>) {
        if !self.app_dir.exists() {
            return (true, None);
        }

        let install_dir = self.app_dir.join("install");
        let has_install_dir = install_dir.exists() && install_dir.is_dir();

        // Scan for admin directory (admin, admin_*, admin[0-9]*, etc.)
        let mut found_admin = None;
        if let Ok(entries) = fs::read_dir(&self.app_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        if name != "admin-api"
                            && name != "admin-dev"
                            && name.starts_with("admin")
                            && path.join("index.php").exists()
                        {
                            // Prefer customized/renamed admin directory (e.g. admin_xyz, admin982a1f)
                            if name != "admin" {
                                found_admin = Some(name.to_string());
                                break;
                            } else if found_admin.is_none() {
                                found_admin = Some(name.to_string());
                            }
                        }
                    }
                }
            }
        }

        let is_setup = has_install_dir || found_admin.is_none();
        (is_setup, found_admin)
    }

    /// Generates active php.ini from template
    pub fn generate_php_ini(&self) -> Result<PathBuf> {
        let template_path = self.config_dir.join("php.ini.template");
        let active_path = self.config_dir.join("php.ini");

        let content = if template_path.exists() {
            fs::read_to_string(&template_path)?
        } else {
            include_str!("../../config/php.ini.template").to_string()
        };

        let root_str = self.root_dir.to_string_lossy().replace('\\', "/");
        let active_content = content.replace("{{ROOT}}", &root_str);

        fs::write(&active_path, active_content)
            .with_context(|| format!("Failed to write active php.ini to {:?}", active_path))?;

        Ok(active_path)
    }

    /// Generates active nginx.conf from template
    pub fn generate_nginx_conf(&self, web_port: u16, php_port: u16) -> Result<PathBuf> {
        let template_path = self.config_dir.join("nginx.conf.template");
        let active_path = self.config_dir.join("nginx.conf");

        let content = if template_path.exists() {
            fs::read_to_string(&template_path)?
        } else {
            include_str!("../../config/nginx.conf.template").to_string()
        };

        let root_str = self.root_dir.to_string_lossy().replace('\\', "/");
        let app_str = self.app_dir.to_string_lossy().replace('\\', "/");

        let active_content = content
            .replace("{{ROOT}}", &root_str)
            .replace("{{APP_DIR}}", &app_str)
            .replace("{{PORT}}", &web_port.to_string())
            .replace("{{PHP_PORT}}", &php_port.to_string());

        fs::write(&active_path, active_content)
            .with_context(|| format!("Failed to write active nginx.conf to {:?}", active_path))?;

        Ok(active_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AppConfig::default();
        assert_eq!(config.web_port, 8080);
        assert_eq!(config.php_port, 9000);
        assert_eq!(config.db_port, 3306);
    }

    #[test]
    fn test_resolve_paths() {
        let paths = EnvPaths::resolve().expect("Should resolve env paths");
        assert!(paths.root_dir.exists());
        assert!(paths.logs_dir.exists());
        assert!(paths.data_dir.exists());
    }

    #[test]
    fn test_generate_configs() {
        let paths = EnvPaths::resolve().expect("Should resolve env paths");
        let php_ini = paths.generate_php_ini().expect("Should generate php.ini");
        assert!(php_ini.exists());

        let nginx_conf = paths
            .generate_nginx_conf(8080, 9000)
            .expect("Should generate nginx.conf");
        assert!(nginx_conf.exists());
    }

    #[test]
    fn test_detect_prestashop_state() {
        let temp_dir = std::env::temp_dir().join(format!("test_ps_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let mut paths = EnvPaths::resolve().expect("Should resolve env paths");
        paths.app_dir = temp_dir.clone();

        // 1. Empty app dir -> setup mode, no admin
        let (is_setup, admin) = paths.detect_prestashop_state();
        assert!(is_setup);
        assert_eq!(admin, None);

        // 2. Install folder + default admin -> setup mode
        fs::create_dir_all(temp_dir.join("install")).unwrap();
        fs::create_dir_all(temp_dir.join("admin")).unwrap();
        fs::write(temp_dir.join("admin/index.php"), "<?php").unwrap();
        let (is_setup, admin) = paths.detect_prestashop_state();
        assert!(is_setup);
        assert_eq!(admin, Some("admin".to_string()));

        // 3. Removed install folder + renamed admin982a1f -> ready mode, renamed admin preferred
        fs::remove_dir_all(temp_dir.join("install")).unwrap();
        fs::create_dir_all(temp_dir.join("admin982a1f")).unwrap();
        fs::write(temp_dir.join("admin982a1f/index.php"), "<?php").unwrap();
        // Also simulate admin-api to ensure it's not chosen
        fs::create_dir_all(temp_dir.join("admin-api")).unwrap();
        fs::write(temp_dir.join("admin-api/index.php"), "<?php").unwrap();

        let (is_setup, admin) = paths.detect_prestashop_state();
        assert!(!is_setup);
        assert_eq!(admin, Some("admin982a1f".to_string()));

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
