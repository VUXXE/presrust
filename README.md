# PrestaShop Portable

[![Dev Build](https://github.com/VUXXE/presrust/actions/workflows/dev-build.yml/badge.svg)](https://github.com/VUXXE/presrust/actions/workflows/dev-build.yml)
[![Release](https://github.com/VUXXE/presrust/actions/workflows/release.yml/badge.svg)](https://github.com/VUXXE/presrust/actions/workflows/release.yml)

Self-contained **PrestaShop 9** that runs without system installs on Windows, Linux, and macOS. One desktop launcher controls everything (Nginx, PHP, MariaDB).

## Download & Run

1. Download the `.zip` / `.tar.xz` for your OS from [Releases](https://github.com/VUXXE/presrust/releases) and extract it anywhere.
2. Run `PrestaShopLauncher` and click **Start Services**.
3. Click **Start Shop Setup**, complete the wizard in your browser (DB credentials are shown in the app), then use **Admin Login**.

No installer, no admin rights — database, logs, and temp files all live inside the app folder, so it can be moved anywhere.

## Features

- **Portable**: everything runs from one folder.
- **Auto-detected admin URL**: works even after the installer renames the admin directory.
- **Live log monitor**: Nginx, PHP, and MariaDB logs with per-service filters.
- **Native desktop app**: Rust + Tauri, OS window controls and app icon included.

## Development

Requires only the Rust toolchain:

```bash
./scripts/setup-local-dev.sh   # optional runtime stubs
cd launcher && cargo run        # run the launcher
cargo fmt --all -- --check && cargo clippy --all-targets -- -D warnings && cargo test
```

Releases are built by GitHub Actions (`git tag vX.Y.Z && git push origin vX.Y.Z`). Details in `prd.md`.

## License

OSL-3.0 / MIT.
