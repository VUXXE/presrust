
# PRD: PrestaShop Portable (Multi-Platform)

**Versi:** 0.4 (Cloud-Native CI/CD Build)

**Status:** Siap Direview

---

## 1. Ringkasan

PrestaShop Portable adalah distribusi paket mandiri (*self-contained*) PrestaShop 9 yang berjalan tanpa instalasi sistem di Windows, Linux, dan macOS (Apple Silicon & Intel). Seluruh layanan (Nginx, PHP FastCGI, MariaDB) dikendalikan melalui launcher desktop berbasis Rust.

**Strategi Pengembangan:**

Lingkungan pengembang lokal (*local machine*) difokuskan murni untuk penulisan kode (`launcher/`), pengujian logika UI, dan penyesuaian konfigurasi template. Seluruh proses kompilasi binary native/cross-platform, perakitan runtime statis, dan *packaging* bundel final dialihkan sepenuhnya ke cloud via **GitHub Actions**.

---

## 2. Tujuan & Batasan

### 2.1 Tujuan

* **Zero-Dependency Setup**: Cukup unduh arsip rilis, ekstrak, dan jalankan launcher.
* **Isolated Portability**: Database (`data/`), log (`logs/`), dan file sementara (`tmp/`) berada di dalam folder proyek; aman dipindah media tanpa konfigurasi ulang.
* **Local Dev Simplicity**: Mesin lokal tidak memerlukan toolchain kompilasi PHP statis, toolchain C cross-platform, atau build engine MariaDB. Cukup toolchain standar Rust untuk pengembangan kode.
* **Automated Cloud Build Factory**: GitHub Actions menangani kompilasi biner Rust (4 target platform), penyiapan runtime statis via caching pintar, *smoke test*, dan publikasi otomatis ke GitHub Releases.
* **Zero-Configuration Admin Redirect**: Launcher mendeteksi perubahan dinamis nama direktori admin pasca-instalasi dan membuka URL login back-office secara otomatis.

### 2.2 Batasan (Non-Tujuan)

* Bukan untuk deployment server produksi publik.
* Layanan hanya mendengarkan pada interface loopback lokal (`127.0.0.1`).
* Pengembang lokal tidak memproduksi binary rilis multi-OS secara manual.

---

## 3. Pembagian Tanggung Jawab: Local vs. GitHub Actions

| Aspek | Local Development (Mesin Pengembang) | GitHub Actions (Cloud CI/CD) |
| --- | --- | --- |
| **Fokus Utama** | Coding logika Rust, desain layout UI, penulisan template config, unit test. | Kompilasi multi-target OS, bundling runtime, kompresi, rilis. |
| **Toolchain Diperlukan** | Rust toolchain (`cargo`, `rustc`), editor kode. | Rust multi-runner, toolchain C/C++, Docker (Linux), `static-php-cli`, cmake. |
| **Penyediaan Runtime** | Menggunakan mock binary atau mengunduh satu paket runtime dev via script download cepat. | Membangun `php-cgi` statis, mengompilasi Nginx, men-strip MariaDB, mengemas PrestaShop. |
| **Hasil Luaran** | Verifikasi fitur launcher via `cargo run`. | 4 paket rilis terkompresi (`.zip` / `.tar.xz`) + file `SHA256SUMS`. |

---

## 4. Arsitektur & Struktur Repositori

### 4.1 Struktur Repositori Kode (Git-Tracked)

Repositori hanya melacak kode sumber, konfigurasi template, dan skrip otomasi CI. **Binary runtime tidak disimpan di Git.**

```
prestashop-portable/
├── .github/
│   └── workflows/
│       ├── dev-build.yml             # Validasi PR / push branch (cek compile & fmt)
│       └── release.yml               # Pipeline penuh kompilasi runtime & packaging
├── ci/
│   ├── build-php.sh                  # Skrip build static-php-cli untuk runner CI
│   ├── build-nginx.sh                # Skrip compile Nginx lean
│   ├── prepare-mariadb.sh            # Skrip stripping MariaDB resmi
│   └── package-bundle.sh             # Skrip perakitan paket final
├── config/
│   ├── nginx.conf
│   └── php.ini.template              # Placeholder {{ROOT}} & {{PORT}}
├── launcher/                         # Kode Sumber Rust (Murni Coding)
│   ├── Cargo.toml
│   ├── Cargo.lock
│   ├── src/
│   │   ├── main.rs
│   │   ├── process/                  # Process manager (Nginx, PHP, MariaDB)
│   │   ├── monitor/                  # Asynchronous log file tailing
│   │   └── ui/                       # Komponen antarmuka GUI (Slint/Iced)
│   └── assets/
├── scripts/
│   └── setup-local-dev.sh            # Opsional: script unduh runtime dummy untuk test local
├── versions.env                      # Definisi versi PrestaShop, PHP, MariaDB, Nginx
└── README.md

```

### 4.2 Struktur Hasil Bundel Rilis (Artifact Output CI)

```
prestashop-portable-release/
├── app/                              # Core PrestaShop 9
├── config/                           # Template konfigurasi
├── runtime/
│   └── {target}/                     # Folder runtime hasil compile CI
│       ├── nginx/
│       ├── php/                      # php-cgi statis
│       └── mariadb/
├── data/mariadb/
├── tmp/{sessions,uploads}/
├── logs/
└── PrestaShopLauncher[.exe]          # Binary launcher hasil compile CI

```

---

## 5. Antarmuka Launcher GUI (Rust)

Desain UI tetap ringkas dan modern, dijalankan di lokal dengan `cargo run` selama masa pengembangan.

### 5.1 State Layanan Berhenti (Idle)

```text
┌──────────────────────────────────────────────┐
│  PrestaShop Portable                   [—][×]│
├──────────────────────────────────────────────┤
│                                              │
│  Status : ○ Berhenti                         │
│  Host   : 127.0.0.1:8080                     │
│                                              │
│  [  ▶ Jalankan Layanan  ]                    │
│                                              │
│  [  Buka Toko  ]           [  Login Admin  ] │
│      (mati)                    (mati)        │
│                                              │
├──────────────────────────────────────────────┤
│  [⚙ Pengaturan]                  v0.4 (Rust) │
└──────────────────────────────────────────────┘

```

### 5.2 State Setup Toko Aktif (Menampilkan Info Database)

```text
┌──────────────────────────────────────────────┐
│  PrestaShop Portable                   [—][×]│
├──────────────────────────────────────────────┤
│                                              │
│  Status : ● Berjalan                         │
│  Host   : http://127.0.0.1:8080              │
│                                              │
│  ┌─ Kredensial Database untuk Setup ───────┐ │
│  │ Host: 127.0.0.1   │ Port: 3306          │ │
│  │ User: root        │ Pass: (kosong)      │ │
│  │ DB  : prestashop  [ Salin Info ]        │ │
│  └─────────────────────────────────────────┘ │
│                                              │
│  [  ■ Hentikan Layanan  ]                    │
│                                              │
│  [ 🌐 Mulai Setup Toko ]   [  Login Admin  ] │
│                                (Terkunci)    │
│  * Selesaikan wizard setup untuk login admin │
│                                              │
├──────────────────────────────────────────────┤
│  [⚙ Pengaturan]       [▼ Tampilkan Log]     │
└──────────────────────────────────────────────┘

```

### 5.3 State Siap Digunakan (Auto-Detect Folder Admin)

```text
┌──────────────────────────────────────────────┐
│  PrestaShop Portable                   [—][×]│
├──────────────────────────────────────────────┤
│                                              │
│  Status : ● Berjalan                         │
│  Host   : http://127.0.0.1:8080              │
│  Admin  : /admin_982a1f/ (terdeteksi)        │
│                                              │
│  [  ■ Hentikan Layanan  ]                    │
│                                              │
│  [ 🛍️ Buka Toko ]          [ 🔐 Login Admin ] │
│                                              │
├──────────────────────────────────────────────┤
│  [⚙ Pengaturan]       [▼ Tampilkan Log]     │
└──────────────────────────────────────────────┘

```

### 5.4 Panel Log Monitor (Drawer / Dedicated View)

```text
┌────────────────────────────────────────────────────────────┐
│  PrestaShop Portable — Log Monitor                   [—][×]│
├────────────────────────────────────────────────────────────┤
│  Status : ● Berjalan   │ Host : http://127.0.0.1:8080      │
│  [  ■ Hentikan Layanan  ] [ 🛍️ Buka Toko ] [ 🔐 Admin ]     │
├────────────────────────────────────────────────────────────┤
│  Filter: [ (•) Semua ] [ ( ) Nginx ] [ ( ) PHP ] [ ( ) DB ]│
│  ┌─ Live Console (logs/*) ───────────────────────────────┐ │
│  │ [06:30:12] [MariaDB] Server socket created 127.0.0.1  │ │
│  │ [06:30:12] [MariaDB] mysqld ready for connections     │ │
│  │ [06:30:13] [PHP-CGI] Listening on 127.0.0.1:9000      │ │
│  │ [06:30:13] [Nginx]   Worker process 4821 started      │ │
│  │ [06:30:18] [Nginx]   GET /index.php 200 OK (0.042s)   │ │
│  │ [06:31:02] [PHP-ERR] Deprecated: Hook filter in ...   │ │
│  │ [06:31:45] [Nginx]   POST /admin_982a1f/login 302     │ │
│  └───────────────────────────────────────────────────────┘ │
│  [✔] Auto-scroll    [ Bersihkan Tampilan ]    [ Salin Log ]│
├────────────────────────────────────────────────────────────┤
│  [⚙ Pengaturan]     [📁 Buka Folder Logs]   [▲ Sembunyikan]│
└────────────────────────────────────────────────────────────┘

```

---

## 6. Kebutuhan Runtime PHP (PrestaShop 9)

Seluruh kompilasi ekstensi ini dieksekusi oleh runner GitHub Actions via `static-php-cli`:

### 6.1 Matriks Versi & Ekstensi Wajib

* **Versi**: PHP 8.4 NTS (PrestaShop 9.1 Ready).
* **Ekstensi Core PrestaShop**: `pdo_mysql`, `pdo`, `curl`, `gd`, `intl`, `zip`, `fileinfo`, `mbstring`, `openssl`, `iconv`, `simplexml`, `dom`.
* **Ekstensi Symfony 6.4 Stack**: `ctype`, `tokenizer`, `xml`, `xmlwriter`, `session`, `filter`, `hash`.
* **Rekomendasi Tambahan**: `opcache` (performa), `mysqli` (kompatibilitas plugin), `sodium` (kriptografi).

### 6.2 Konfigurasi Dinamis `php.ini`

Launcher Rust secara otomatis mengonversi `php.ini.template` menjadi `php.ini` aktif pada setiap startup:

* Membatasi konsumsi memori: `memory_limit = 512M`.
* Menyesuaikan batas upload: `upload_max_filesize = 64M`, `post_max_size = 64M`.
* Mengisolasi file temporer & sesi: `session.save_path` dan `upload_tmp_dir` diarahkan ke `tmp/`.
* Mengisolasi logging error: `error_log` diarahkan ke `logs/php_errors.log`.

---

## 7. Desain Pipeline GitHub Actions (The Build Factory)

Semua proses berat dipindahkan ke cloud dengan skema berikut:

### 7.1 Matriks Runner Multi-Platform

| Target Arsitektur | Runner GitHub | Komponen yang Dibuat |
| --- | --- | --- |
| `windows-x86_64` | `windows-2025` | Build Rust Launcher (`.exe`), unduh PHP NTS & Nginx resmi, stripping MariaDB Windows. |
| `linux-x86_64` | `ubuntu-24.04` | Build Rust Launcher (ELF), compile static `php-cgi` & Nginx via GCC/musl, stripping MariaDB Linux. |
| `macos-arm64` | `macos-15` | Native Apple Silicon build untuk Rust Launcher, compile static `php-cgi`, Nginx, dan MariaDB. |
| `macos-x86_64` | `macos-15-intel` | Native Intel Mac build untuk seluruh komponen (tersedia hingga akhir masa operasional runner Intel). |

### 7.2 Strategi Caching (Mempercepat Build Cloud)

Kompilasi PHP statis dan MariaDB membutuhkan waktu 15–30 menit jika di-build dari nol. CI menerapkan `actions/cache` berdasarkan hash file `versions.env`:

* Jika versi pada `versions.env` tidak berubah, runtime ditarik langsung dari cache CI (< 30 detik).
* Kompilasi biner runtime hanya terpicu jika ada perubahan versi dependensi atau file skrip build di `ci/`.
* Job compile Rust Launcher selalu berjalan cepat memanfaatkan cache direktori `target/` dari Cargo.

### 7.3 Workflow Rilis (`.github/workflows/release.yml`)

```yaml
name: release-pipeline

on:
  push:
    tags: ["v*"]
  workflow_dispatch:

jobs:
  build-binaries:
    name: Build & Bundle (${{ matrix.target }})
    strategy:
      fail-fast: false
      matrix:
        include:
          - { target: windows-x86_64, os: windows-2025, ext: zip }
          - { target: linux-x86_64,   os: ubuntu-24.04, ext: tar.xz }
          - { target: macos-arm64,    os: macos-15,     ext: tar.xz }
          - { target: macos-x86_64,   os: macos-15-intel, ext: tar.xz }
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4

      # 1. Setup Rust Toolchain
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
        with:
          workspaces: "launcher -> target"

      # 2. Cache Runtime Binaries (PHP, Nginx, MariaDB)
      - name: Cache Runtime Components
        id: cache-runtime
        uses: actions/cache@v4
        with:
          path: build/runtime/${{ matrix.target }}
          key: runtime-${{ matrix.target }}-${{ hashFiles('versions.env', 'ci/**') }}

      # 3. Build / Fetch Runtimes (Hanya jalan jika cache miss)
      - name: Build Runtime Binaries
        if: steps.cache-runtime.outputs.cache-hit != 'true'
        run: ./ci/build-runtime.sh ${{ matrix.target }}
        shell: bash

      # 4. Compile Rust Launcher
      - name: Build Launcher Binary
        run: |
          cd launcher
          cargo build --release --locked
        shell: bash

      # 5. Rakit Paket & PrestaShop Core
      - name: Package Distribution
        run: ./ci/package-bundle.sh ${{ matrix.target }} ${{ matrix.ext }}
        shell: bash

      # 6. Automated Smoke Test di Runner
      - name: Run Smoke Tests
        run: ./ci/smoke-test.sh ${{ matrix.target }}
        shell: bash

      # 7. Upload Artifacts
      - uses: actions/upload-artifact@v4
        with:
          name: prestashop-portable-${{ matrix.target }}
          path: dist/*.${{ matrix.ext }}

  publish-release:
    needs: build-binaries
    runs-on: ubuntu-24.04
    permissions:
      contents: write
    steps:
      - uses: actions/download-artifact@v4
        with:
          path: release-assets
          merge-multiple: true
      - name: Compute Checksums
        run: |
          cd release-assets
          sha256sum * > SHA256SUMS
      - name: Publish to GitHub Releases
        uses: softprops/action-gh-release@v2
        if: startsWith(github.ref, 'refs/tags/v')
        with:
          files: release-assets/*
          generate_release_notes: true

```

---

## 8. Alur Kerja Developer (Local Development Workflow)

1. **Coding di Mesin Lokal**:
* Developer hanya fokus menyunting kode di direktori `launcher/` (Rust).
* Menjalankan antarmuka secara instan menggunakan perintah standar:
```bash
cd launcher
cargo run

```


* Skrip pembantu lokal `scripts/setup-local-dev.sh` dapat digunakan jika ingin mengunduh biner *dummy/stub* runtime untuk menguji fitur *spawn child process*.


2. **Push Perubahan**:
* Developer melakukan commit dan push kode launcher atau perbaikan template konfigurasi ke GitHub.


3. **Automasi Cloud**:
* GitHub Actions memvalidasi kompilasi lintas platform.
* Saat tag versi (`v0.4.0`) di-push, GitHub Actions otomatis merakit seluruh komponen biner dari cache/source dan merilis paket siap pakai di GitHub Releases.
