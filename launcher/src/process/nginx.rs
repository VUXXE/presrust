use crate::config::EnvPaths;
use anyhow::{bail, Context, Result};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};

pub struct NginxService {
    child: Option<Child>,
    pub port: u16,
    pub php_port: u16,
}

impl NginxService {
    pub fn new(port: u16, php_port: u16) -> Self {
        Self {
            child: None,
            port,
            php_port,
        }
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
            paths.runtime_dir.join("linux-x86_64/nginx/sbin/nginx"),
            paths.runtime_dir.join("windows-x86_64/nginx/nginx.exe"),
            paths.runtime_dir.join("macos-arm64/nginx/sbin/nginx"),
            paths.runtime_dir.join("macos-x86_64/nginx/sbin/nginx"),
            paths.runtime_dir.join("nginx/sbin/nginx"),
            paths.runtime_dir.join("nginx/nginx.exe"),
        ];

        for path in &candidates {
            if path.exists() {
                return Ok(path.clone());
            }
        }

        if let Ok(entries) = std::fs::read_dir(&paths.runtime_dir) {
            for entry in entries.flatten() {
                let bin = entry.path().join("nginx/sbin/nginx");
                if bin.exists() {
                    return Ok(bin);
                }
                let w_bin = entry.path().join("nginx/nginx.exe");
                if w_bin.exists() {
                    return Ok(w_bin);
                }
            }
        }

        for name in &["nginx", "nginx.exe"] {
            if let Ok(path) = which::which(name) {
                return Ok(path);
            }
        }

        bail!("Nginx binary not found in runtime directory or system PATH")
    }

    pub fn start(&mut self, paths: &EnvPaths) -> Result<()> {
        if self.is_running() {
            return Ok(());
        }

        let bin = Self::find_binary(paths)?;
        let nginx_conf = paths.generate_nginx_conf(self.port, self.php_port)?;

        let mut cmd = Command::new(&bin);
        cmd.arg("-p")
            .arg(paths.root_dir.to_string_lossy().to_string())
            .arg("-c")
            .arg(nginx_conf.to_string_lossy().to_string())
            .stdout(Stdio::null())
            .stderr(Stdio::null());

        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x08000000);
        }

        let child = cmd
            .spawn()
            .with_context(|| format!("Failed to spawn Nginx process {:?}", bin))?;

        self.child = Some(child);
        Ok(())
    }

    pub fn stop(&mut self) -> Result<()> {
        if let Some(mut child) = self.child.take() {
            #[cfg(unix)]
            {
                unsafe {
                    libc::kill(child.id() as libc::pid_t, libc::SIGQUIT);
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

impl Drop for NginxService {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}
