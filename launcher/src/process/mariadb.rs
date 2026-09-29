use crate::config::EnvPaths;
use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

pub struct MariaDbService {
    child: Option<Child>,
    pub port: u16,
}

impl MariaDbService {
    pub fn new(port: u16) -> Self {
        Self { child: None, port }
    }

    pub fn is_running(&mut self) -> bool {
        if let Some(ref mut child) = self.child {
            match child.try_wait() {
                Ok(None) => true,
                _ => {
                    self.child = None;
                    false
                }
            }
        } else {
            false
        }
    }

    pub fn find_binary(paths: &EnvPaths) -> Result<PathBuf> {
        let candidates = [
            // Runtime directory search
            paths.runtime_dir.join("linux-x86_64/mariadb/bin/mariadbd"),
            paths
                .runtime_dir
                .join("windows-x86_64/mariadb/bin/mysqld.exe"),
            paths
                .runtime_dir
                .join("windows-x86_64/mariadb/bin/mariadbd.exe"),
            paths.runtime_dir.join("macos-arm64/mariadb/bin/mariadbd"),
            paths.runtime_dir.join("macos-x86_64/mariadb/bin/mariadbd"),
            paths.runtime_dir.join("mariadb/bin/mariadbd"),
            paths.runtime_dir.join("mariadb/bin/mysqld"),
        ];

        for path in &candidates {
            if path.exists() {
                return Ok(path.clone());
            }
        }

        // Check any matching runtime/* subdirectory
        if let Ok(entries) = std::fs::read_dir(&paths.runtime_dir) {
            for entry in entries.flatten() {
                let m_bin = entry.path().join("mariadb/bin/mariadbd");
                if m_bin.exists() {
                    return Ok(m_bin);
                }
                let w_bin = entry.path().join("mariadb/bin/mysqld.exe");
                if w_bin.exists() {
                    return Ok(w_bin);
                }
            }
        }

        // Dev fallback: PATH lookup
        for name in &["mariadbd", "mysqld"] {
            if let Ok(path) = which::which(name) {
                return Ok(path);
            }
        }

        bail!("MariaDB binary not found in runtime directory or system PATH")
    }

    pub fn start(&mut self, paths: &EnvPaths) -> Result<()> {
        if self.is_running() {
            return Ok(());
        }

        let bin = Self::find_binary(paths)?;
        let datadir = paths.data_dir.join("mariadb");
        let error_log = paths.logs_dir.join("mariadb_error.log");

        // First-time database init if mysql system database directory doesn't exist
        if !datadir.join("mysql").exists() {
            let _ = Self::initialize_db(paths, &datadir);
        }

        let mut cmd = Command::new(&bin);
        // --no-defaults MUST be the first argument to prevent loading host system /etc/my.cnf.d/*.cnf
        cmd.arg("--no-defaults")
            .arg(format!("--datadir={}", datadir.to_string_lossy()))
            .arg(format!("--port={}", self.port))
            .arg("--bind-address=127.0.0.1")
            .arg(format!("--log-error={}", error_log.to_string_lossy()))
            .arg(format!(
                "--socket={}",
                paths.tmp_dir.join("mysql.sock").to_string_lossy()
            ))
            .arg(format!(
                "--pid-file={}",
                paths.tmp_dir.join("mariadb.pid").to_string_lossy()
            ))
            .arg(format!("--tmpdir={}", paths.tmp_dir.to_string_lossy()))
            .arg("--default-storage-engine=InnoDB")
            .arg("--skip-networking=0")
            .stdout(Stdio::null())
            .stderr(Stdio::null());

        // Only set --basedir if it's a real bundled MariaDB (not a symlink to /usr or system)
        if let Ok(real_path) = bin.canonicalize() {
            if !real_path.starts_with("/usr") {
                if let Some(base) = real_path.parent().and_then(|p| p.parent()) {
                    if base.join("share/english").exists() || base.join("share/charsets").exists() {
                        cmd.arg(format!("--basedir={}", base.to_string_lossy()));
                    }
                }
            }
        }

        let init_sql = paths.tmp_dir.join("init_prestashop.sql");
        let _ = std::fs::write(
            &init_sql,
            "CREATE DATABASE IF NOT EXISTS `prestashop` CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;\n",
        );
        cmd.arg(format!("--init-file={}", init_sql.to_string_lossy()));

        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x08000000);
        }

        let child = cmd
            .spawn()
            .with_context(|| format!("Failed to spawn MariaDB process {:?}", bin))?;

        self.child = Some(child);
        Ok(())
    }

    fn initialize_db(paths: &EnvPaths, datadir: &Path) -> Result<()> {
        // Search for install-db binary
        let mut install_bin = None;
        for sub in &[
            "linux-x86_64",
            "windows-x86_64",
            "macos-arm64",
            "macos-x86_64",
            "",
        ] {
            let p1 = paths
                .runtime_dir
                .join(sub)
                .join("mariadb/bin/mariadb-install-db");
            let p2 = paths
                .runtime_dir
                .join(sub)
                .join("mariadb/bin/mysql_install_db.exe");
            let p3 = paths
                .runtime_dir
                .join(sub)
                .join("mariadb/scripts/mysql_install_db");
            if p1.exists() {
                install_bin = Some(p1);
                break;
            }
            if p2.exists() {
                install_bin = Some(p2);
                break;
            }
            if p3.exists() {
                install_bin = Some(p3);
                break;
            }
        }

        if install_bin.is_none() {
            if let Ok(path) = which::which("mariadb-install-db") {
                install_bin = Some(path);
            } else if let Ok(path) = which::which("mysql_install_db") {
                install_bin = Some(path);
            }
        }

        if let Some(installer) = install_bin {
            let mut init_cmd = Command::new(&installer);
            init_cmd
                .arg("--no-defaults")
                .arg(format!("--datadir={}", datadir.to_string_lossy()))
                .arg("--auth-root-authentication-method=normal")
                .arg("--skip-test-db");

            if let Ok(real_path) = installer.canonicalize() {
                if !real_path.starts_with("/usr") {
                    if let Some(base) = real_path.parent().and_then(|p| p.parent()) {
                        if base.join("share/english").exists()
                            || base.join("share/charsets").exists()
                        {
                            init_cmd.arg(format!("--basedir={}", base.to_string_lossy()));
                        }
                    }
                }
            }

            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                init_cmd.creation_flags(0x08000000);
            }

            let _ = init_cmd.status();
        }
        Ok(())
    }

    pub fn stop(&mut self) -> Result<()> {
        if let Some(mut child) = self.child.take() {
            #[cfg(unix)]
            {
                unsafe {
                    libc::kill(child.id() as libc::pid_t, libc::SIGTERM);
                }
            }
            #[cfg(windows)]
            {
                let _ = Command::new("taskkill")
                    .args(["/F", "/T", "/PID", &child.id().to_string()])
                    .status();
            }

            // Wait up to 3 seconds for clean shutdown
            for _ in 0..30 {
                if let Ok(Some(_)) = child.try_wait() {
                    return Ok(());
                }
                std::thread::sleep(std::time::Duration::from_millis(100));
            }

            let _ = child.kill();
            let _ = child.wait();
        }
        Ok(())
    }
}

impl Drop for MariaDbService {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}
