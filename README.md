# PrestaShop Portable (Multi-Platform)

[![Dev Build](https://github.com/your-org/prestashop-portable/actions/workflows/dev-build.yml/badge.svg)](https://github.com/your-org/prestashop-portable/actions/workflows/dev-build.yml)
[![Release](https://github.com/your-org/prestashop-portable/actions/workflows/release.yml/badge.svg)](https://github.com/your-org/prestashop-portable/actions/workflows/release.yml)

Distribusi mandiri (*self-contained / zero-dependency*) **PrestaShop 9** yang berjalan tanpa instalasi sistem di Windows, Linux, dan macOS (Apple Silicon & Intel). Seluruh layanan (Nginx, PHP FastCGI statis, dan MariaDB) dikendalikan melalui launcher desktop modern berbasis Rust.

---

## Fitur Utama

- **Zero-Dependency**: Cukup unduh arsip rilis, ekstrak, dan jalankan launcher.
- **Isolasi Penuh**: Database (`data/`), log (`logs/`), dan file sementara (`tmp/`) berada di dalam folder aplikasi; aman dipindahkan antar media tanpa konfigurasi ulang.
- **Admin Redirect Otomatis**: Launcher mendeteksi perubahan dinamis nama direktori admin pasca-instalasi dan membuka URL login back-office secara otomatis.
- **Live Log Monitor**: Pemantauan log terpusat untuk Nginx, PHP-CGI, dan MariaDB dengan filter per-layanan, auto-scroll, dan salin log instan.
- **Cloud-Native CI/CD Build Factory**: Mesin lokal difokuskan murni untuk coding logika launcher di Rust. Kompilasi binary native/cross-platform dan runtime statis dilakukan otomatis di GitHub Actions.

---

## Struktur Repositori

```text
prestashop-portable/
├── .github/
│   └── workflows/
│       ├── dev-build.yml             # CI validasi formatting, clippy, dan unit test
│       └── release.yml               # Pipeline rilis multi-OS dan packaging
├── ci/
│   ├── build-php.sh                  # Skrip build static-php-cli per runner
│   ├── build-nginx.sh                # Skrip compile Nginx lean
│   ├── prepare-mariadb.sh            # Skrip stripping MariaDB resmi
│   ├── build-runtime.sh              # Orchestrator build runtime
│   ├── package-bundle.sh             # Skrip perakitan paket final
│   └── smoke-test.sh                 # Pengujian biner otomatis di CI
├── config/
│   ├── nginx.conf                    # Konfigurasi aktif (dihasilkan otomatis)
│   ├── nginx.conf.template           # Template konfigurasi Nginx portable
│   └── php.ini.template              # Template konfigurasi PHP 8.4 portable
├── launcher/                         # Kode Sumber Rust Desktop Launcher
│   ├── Cargo.toml
│   ├── build.rs
│   ├── ui/
│   │   └── launcher.slint            # Definisi UI Slint
│   └── src/
│       ├── main.rs                   # Entrypoint & event loop
│       ├── config.rs                 # Deteksi lingkungan & templating
│       ├── monitor/                  # Asynchronous log tailing
│       └── process/                  # Manajer proses (MariaDB, PHP, Nginx)
├── scripts/
│   └── setup-local-dev.sh            # Setup runtime stub/link lokal untuk dev cepat
├── versions.env                      # Definisi versi komponen (PHP, PrestaShop, dll)
├── prd.md                            # Product Requirements Document
└── README.md
```

---

## Panduan Pengembangan Lokal (Local Development)

Pengembang hanya memerlukan toolchain standar Rust (`cargo`, `rustc`).

### 1. Menyiapkan Lingkungan Lokal
Jalankan skrip pembantu untuk membuat stub runtime atau menautkan binary sistem lokal:
```bash
./scripts/setup-local-dev.sh
```

### 2. Menjalankan Launcher
```bash
cd launcher
cargo run
```

### 3. Validasi Kode & Linting
```bash
cd launcher
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
```

---

## Pipeline Rilis Otomatis (GitHub Actions)

Kompilasi multi-platform dan perakitan bundel final ditangani sepenuhnya oleh GitHub Actions:

| Target Platform | Runner CI | Format Arsip |
| --- | --- | --- |
| `windows-x86_64` | `windows-2025` | `.zip` |
| `linux-x86_64` | `ubuntu-24.04` | `.tar.xz` |
| `macos-arm64` | `macos-15` (Apple Silicon) | `.tar.xz` |
| `macos-x86_64` | `macos-15-intel` (Intel) | `.tar.xz` |

Untuk memicu build rilis otomatis:
```bash
git tag v0.4.0
git push origin v0.4.0
```

---

## Lisensi

Proyek ini dilisensikan di bawah Open Software License (OSL 3.0) / MIT.
