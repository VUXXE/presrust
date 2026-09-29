use crate::config::EnvPaths;
use anyhow::{bail, Context, Result};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};

pub struct PhpService {
    child: Option<Child>,
    pub port: u16,
}

impl PhpService {
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
            paths.runtime_dir.join("linux-x86_64/php/php-cgi"),
            paths.runtime_dir.join("windows-x86_64/php/php-cgi.exe"),
            paths.runtime_dir.join("macos-arm64/php/php-cgi"),
            paths.runtime_dir.join("macos-x86_64/php/php-cgi"),
            paths.runtime_dir.join("php/php-cgi"),
            paths.runtime_dir.join("php/php-cgi.exe"),
        ];

        for path in &candidates {
            if path.exists() {
                return Ok(path.clone());
            }
        }

        if let Ok(entries) = std::fs::read_dir(&paths.runtime_dir) {
            for entry in entries.flatten() {
                let bin = entry.path().join("php/php-cgi");
                if bin.exists() {
                    return Ok(bin);
                }
                let w_bin = entry.path().join("php/php-cgi.exe");
                if w_bin.exists() {
                    return Ok(w_bin);
                }
            }
        }

        for name in &["php-cgi", "php-cgi.exe"] {
            if let Ok(path) = which::which(name) {
                return Ok(path);
            }
        }

        bail!("PHP-CGI binary not found in runtime directory or system PATH")
    }

    pub fn start(&mut self, paths: &EnvPaths) -> Result<()> {
        if self.is_running() {
            return Ok(());
        }

        let bin = Self::find_binary(paths)?;
        let php_ini = paths.generate_php_ini()?;

        let mut cmd = Command::new(&bin);
        cmd.arg("-b")
            .arg(format!("127.0.0.1:{}", self.port))
            .arg("-c")
            .arg(php_ini.to_string_lossy().to_string())
            .env("PHP_FCGI_CHILDREN", "4")
            .env("PHP_FCGI_MAX_REQUESTS", "1000")
            .env("PHP_INI_SCAN_DIR", "")
            .stdout(Stdio::null())
            .stderr(Stdio::null());

        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x08000000);
        }

        #[cfg(target_os = "linux")]
        unsafe {
            use std::os::unix::process::CommandExt;
            cmd.pre_exec(|| {
                libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM);
                Ok(())
            });
        }

        let child = cmd
            .spawn()
            .with_context(|| format!("Failed to spawn PHP-CGI process {:?}", bin))?;

        let _ = std::fs::write(paths.tmp_dir.join("php.pid"), child.id().to_string());

        self.child = Some(child);
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

            for _ in 0..20 {
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

impl Drop for PhpService {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}
