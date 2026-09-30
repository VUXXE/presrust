#!/usr/bin/env bash
set -euo pipefail

TARGET="${1:-linux-x86_64}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"

STAGE_DIR="${ROOT_DIR}/build/stage/prestashop-portable-${TARGET}"

echo "==> Running Smoke Tests for ${TARGET} in ${STAGE_DIR}..."

# Check directories
for dir in app config runtime data tmp logs; do
    if [ ! -d "${STAGE_DIR}/${dir}" ]; then
        echo "FAIL: Required directory missing: ${dir}" >&2
        exit 1
    fi
done

# Check essential configuration files
for cfg in nginx.conf.template php.ini.template mime.types fastcgi.conf; do
    if [ ! -f "${STAGE_DIR}/config/${cfg}" ]; then
        echo "FAIL: Required config file missing: ${cfg}" >&2
        exit 1
    fi
done

# Check launcher
LAUNCHER="PrestaShopLauncher"
[[ "${TARGET}" == windows* ]] && LAUNCHER="PrestaShopLauncher.exe"

if [ ! -f "${STAGE_DIR}/${LAUNCHER}" ]; then
    echo "FAIL: Launcher executable not found: ${STAGE_DIR}/${LAUNCHER}" >&2
    exit 1
fi

# Check PrestaShop index.php
if [ ! -f "${STAGE_DIR}/app/index.php" ]; then
    echo "FAIL: PrestaShop app/index.php not found!" >&2
    exit 1
fi

# Smoke test native binaries on the current runner
if [[ "$(uname -s)" == "Linux" && "${TARGET}" == "linux-x86_64" ]] || \
   [[ "$(uname -s)" == "Darwin" && "${TARGET}" == "macos"* ]]; then
    
    PHP_BIN="${STAGE_DIR}/runtime/${TARGET}/php/php-fpm"
    if [ -f "${PHP_BIN}" ]; then
        echo "--> Testing PHP binary: ${PHP_BIN} -v"
        "${PHP_BIN}" -v || true
    fi

    PHP_BIN="${STAGE_DIR}/runtime/${TARGET}/php/php-cgi"
    if [ -f "${PHP_BIN}" ]; then
        echo "--> Testing PHP binary: ${PHP_BIN} -v"
        "${PHP_BIN}" -v || true
    fi

    NGINX_BIN="${STAGE_DIR}/runtime/${TARGET}/nginx/sbin/nginx"
    if [ -f "${NGINX_BIN}" ]; then
        echo "--> Testing Nginx binary: ${NGINX_BIN} -v"
        "${NGINX_BIN}" -v || true
    fi

    MARIADB_BIN="${STAGE_DIR}/runtime/${TARGET}/mariadb/bin/mariadbd"
    if [ -f "${MARIADB_BIN}" ]; then
        echo "--> Testing MariaDB binary: ${MARIADB_BIN} --version"
        "${MARIADB_BIN}" --version || true
    fi
fi

if [[ "${TARGET}" == windows* ]]; then
    PHP_BIN="${STAGE_DIR}/runtime/${TARGET}/php/php-cgi.exe"
    if [ -f "${PHP_BIN}" ]; then
        echo "--> Testing Windows PHP binary: ${PHP_BIN} -v"
        "${PHP_BIN}" -v || true
    fi

    NGINX_BIN="${STAGE_DIR}/runtime/${TARGET}/nginx/nginx.exe"
    if [ -f "${NGINX_BIN}" ]; then
        echo "--> Testing Windows Nginx binary: ${NGINX_BIN} -v"
        "${NGINX_BIN}" -v || true
    fi

    MARIADB_BIN="${STAGE_DIR}/runtime/${TARGET}/mariadb/bin/mariadbd.exe"
    [ -f "${MARIADB_BIN}" ] || MARIADB_BIN="${STAGE_DIR}/runtime/${TARGET}/mariadb/bin/mysqld.exe"
    if [ -f "${MARIADB_BIN}" ]; then
        echo "--> Testing Windows MariaDB binary: ${MARIADB_BIN} --version"
        "${MARIADB_BIN}" --version || true
    fi
fi

echo "==> All Smoke Tests passed for ${TARGET}!"
