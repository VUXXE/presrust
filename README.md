# PrestaShop Portable

[![Dev Build](https://github.com/VUXXE/prestashop-portable-rust/actions/workflows/dev-build.yml/badge.svg)](https://github.com/VUXXE/prestashop-portable-rust/actions/workflows/dev-build.yml)
[![Release](https://github.com/VUXXE/prestashop-portable-rust/actions/workflows/release.yml/badge.svg)](https://github.com/VUXXE/prestashop-portable-rust/actions/workflows/release.yml)

Self-contained **PrestaShop 9** distribution that runs locally without system installs or root privileges on **Windows**, **Linux**, and **macOS** (Apple Silicon & Intel).

A lightweight native desktop launcher built with Rust + Tauri orchestrates all required runtimes (**Nginx 1.26**, **PHP 8.4 FastCGI / FPM**, and **MariaDB 11.4**).

---

## Download & Quick Start

1. **Download**: Grab the `.zip` archive for your platform from [Latest Releases](https://github.com/VUXXE/prestashop-portable-rust/releases) and extract it anywhere (e.g. desktop, documents, or USB drive).
2. **Launch**: Open `PrestaShopLauncher` (or `PrestaShopLauncher.exe` on Windows).
3. **Start Services**: Click **Start Services** to boot MariaDB, PHP, and Nginx.
4. **Setup Shop**: Click **Start Shop Setup** to launch the browser wizard:
   - **Database Server**: `127.0.0.1` (Port `3306`)
   - **Database Name**: `prestashop`
   - **Database Login**: `root`
   - **Database Password**: *(leave blank)*
5. **Access Back-Office**: Once the wizard finishes, the launcher automatically detects your randomized admin URL and enables the **Admin Login** button.

---

## Features

- **100% Portable**: All databases (`data/`), configurations (`config/`), temp uploads/sessions (`tmp/`), and logs (`logs/`) live inside the bundle.
- **Smart Admin Detection**: Automatically identifies the admin directory even after PrestaShop renames it for security.
- **Reinstall & Reset**: Built-in reset button to cleanly wipe the database and re-enable setup if an installation is interrupted.
- **Self-Healing Environment**: Automatically guarantees runtime permissions and required Symfony `.env` configuration files.
- **Real-Time Log Stream**: Monitor Nginx access/error, PHP errors, and MariaDB logs directly from the desktop UI.
- **Native GUI**: Built on Rust and Tauri with minimal memory footprint and zero external dependencies.

---

## Directory Structure

```text
prestashop-portable/
├── PrestaShopLauncher       # Native launcher GUI
├── app/                     # PrestaShop 9 core application
├── config/                  # Nginx, PHP, and FastCGI templates
├── data/                    # MariaDB database data directory
├── logs/                    # Nginx, PHP, and MariaDB log files
├── runtime/                 # Platform-specific isolated binaries
└── tmp/                     # Sessions, uploads, and temporary files
```

---

## Local Development

Only the Rust toolchain is required to develop and build the launcher locally:

```bash
# Optional runtime stubs or system links for local development
./scripts/setup-local-dev.sh

# Run the launcher in development mode
cd launcher
cargo run

# Code verification & tests
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
```

Cross-platform multi-architecture release packages and runtime builds are fully automated via GitHub Actions.

---

## License

OSL-3.0 / MIT.
