#!/usr/bin/env bash
set -euo pipefail

TARGET="${1:-linux-x86_64}"
OUTPUT_DIR="${2:-build/runtime/${TARGET}/mariadb}"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"

if [ -f "${ROOT_DIR}/versions.env" ]; then
    # shellcheck disable=SC1091
    source "${ROOT_DIR}/versions.env"
fi

MARIADB_VERSION="${MARIADB_VERSION:-11.4.5}"

echo "==> Preparing stripped MariaDB ${MARIADB_VERSION} for ${TARGET}..."
mkdir -p "${OUTPUT_DIR}"

BASE_URL="https://archive.mariadb.org/mariadb-${MARIADB_VERSION}"

case "${TARGET}" in
    windows-x86_64)
        echo "--> Downloading official Windows MariaDB ${MARIADB_VERSION} zip..."
        WIN_URL="${BASE_URL}/winx64-packages/mariadb-${MARIADB_VERSION}-winx64.zip"
        TEMP_ZIP="$(mktemp --suffix=.zip 2>/dev/null || mktemp).zip"
        curl -fsSL -o "${TEMP_ZIP}" "${WIN_URL}"

        EXTRACT_TMP="$(mktemp -d)"
        unzip -q "${TEMP_ZIP}" -d "${EXTRACT_TMP}"
        SRC_DIR="$(find "${EXTRACT_TMP}" -mindepth 1 -maxdepth 1 -type d | head -n 1)"

        # Copy essential binaries
        mkdir -p "${OUTPUT_DIR}/bin" "${OUTPUT_DIR}/share"
        for bin in mysqld.exe mariadbd.exe mysql.exe mariadb.exe mysqladmin.exe mariadb-admin.exe mysql_install_db.exe; do
            if [ -f "${SRC_DIR}/bin/${bin}" ]; then
                cp "${SRC_DIR}/bin/${bin}" "${OUTPUT_DIR}/bin/"
            fi
        done
        # Copy required DLLs from bin
        cp "${SRC_DIR}"/bin/*.dll "${OUTPUT_DIR}/bin/" 2>/dev/null || true

        # Copy VC++ runtime DLLs if running on Windows host
        VC_DLLS=(vcruntime140.dll vcruntime140_1.dll msvcp140.dll msvcp140_1.dll msvcp140_2.dll msvcp140_codecvt_ids.dll vcomp140.dll concrt140.dll)
        for sys_dir in "/c/Windows/System32" "/c/Windows/SysWOW64" "C:/Windows/System32" "${WINDIR:-}/System32"; do
            if [ -n "${sys_dir}" ] && [ -f "${sys_dir}/vcruntime140.dll" ]; then
                for dll in "${VC_DLLS[@]}"; do
                    if [ -f "${sys_dir}/${dll}" ]; then
                        cp -f "${sys_dir}/${dll}" "${OUTPUT_DIR}/bin/" 2>/dev/null || true
                    fi
                done
                break
            fi
        done

        # Copy essential share files
        if [ -d "${SRC_DIR}/share" ]; then
            cp -r "${SRC_DIR}/share" "${OUTPUT_DIR}/"
            # Remove non-english translation files to save space
            find "${OUTPUT_DIR}/share" -mindepth 1 -maxdepth 1 -type d ! -name "english" ! -name "charsets" -exec rm -rf {} + 2>/dev/null || true
        fi

        rm -rf "${EXTRACT_TMP}" "${TEMP_ZIP}"
        ;;

    linux-x86_64)
        echo "--> Downloading official generic Linux MariaDB ${MARIADB_VERSION} tar.gz..."
        # Generic x86_64 glibc tarball
        LINUX_URL="${BASE_URL}/bintar-linux-systemd-x86_64/mariadb-${MARIADB_VERSION}-linux-systemd-x86_64.tar.gz"
        TEMP_TAR="$(mktemp --suffix=.tar.gz 2>/dev/null || mktemp).tar.gz"
        if ! curl -fsSL -o "${TEMP_TAR}" "${LINUX_URL}"; then
            # Fallback to non-systemd generic bintar
            LINUX_URL="${BASE_URL}/bintar-linux-x86_64/mariadb-${MARIADB_VERSION}-linux-x86_64.tar.gz"
            curl -fsSL -o "${TEMP_TAR}" "${LINUX_URL}"
        fi

        EXTRACT_TMP="$(mktemp -d)"
        tar -xzf "${TEMP_TAR}" -C "${EXTRACT_TMP}"
        SRC_DIR="$(find "${EXTRACT_TMP}" -mindepth 1 -maxdepth 1 -type d | head -n 1)"

        mkdir -p "${OUTPUT_DIR}/bin" "${OUTPUT_DIR}/share" "${OUTPUT_DIR}/lib"
        for bin in mariadbd mysqld mariadb mysql mariadb-admin mysqladmin mariadb-install-db mysql_install_db my_print_defaults resolveip; do
            if [ -f "${SRC_DIR}/bin/${bin}" ]; then
                cp "${SRC_DIR}/bin/${bin}" "${OUTPUT_DIR}/bin/"
                strip "${OUTPUT_DIR}/bin/${bin}" 2>/dev/null || true
            fi
        done
        chmod +x "${OUTPUT_DIR}/bin"/* 2>/dev/null || true

        # Scripts
        if [ -f "${SRC_DIR}/scripts/mysql_install_db" ]; then
            mkdir -p "${OUTPUT_DIR}/scripts"
            cp "${SRC_DIR}/scripts/mysql_install_db" "${OUTPUT_DIR}/scripts/"
            chmod +x "${OUTPUT_DIR}/scripts/mysql_install_db"
        fi

        # Share
        if [ -d "${SRC_DIR}/share" ]; then
            cp -r "${SRC_DIR}/share" "${OUTPUT_DIR}/"
            find "${OUTPUT_DIR}/share" -mindepth 1 -maxdepth 1 -type d ! -name "english" ! -name "charsets" -exec rm -rf {} + 2>/dev/null || true
        fi

        # Libs
        if [ -d "${SRC_DIR}/lib" ]; then
            cp -r "${SRC_DIR}"/lib/*.so* "${OUTPUT_DIR}/lib/" 2>/dev/null || true
        fi

        rm -rf "${EXTRACT_TMP}" "${TEMP_TAR}"
        ;;

    macos-arm64|macos-x86_64)
        echo "--> macOS MariaDB: preparing binaries..."
        mkdir -p "${OUTPUT_DIR}/bin" "${OUTPUT_DIR}/share"

        # On macOS runners, install MariaDB via Homebrew if mariadbd is missing
        if ! command -v mariadbd >/dev/null 2>&1; then
            echo "--> Installing MariaDB via Homebrew on macOS runner..."
            HOMEBREW_NO_AUTO_UPDATE=1 brew install mariadb || true
        fi

        M_DIR=""
        if command -v brew >/dev/null 2>&1; then
            M_DIR="$(brew --prefix mariadb 2>/dev/null || true)"
        fi

        if [ -n "${M_DIR}" ] && [ -d "${M_DIR}" ]; then
            echo "--> Extracting from Homebrew MariaDB at ${M_DIR}..."
            for b in mariadbd mariadb mariadb-install-db mysqladmin mariadb-admin; do
                if [ -f "${M_DIR}/bin/${b}" ]; then
                    cp "${M_DIR}/bin/${b}" "${OUTPUT_DIR}/bin/"
                fi
            done
            if [ -d "${M_DIR}/share/mariadb" ]; then
                cp -r "${M_DIR}/share/mariadb" "${OUTPUT_DIR}/share/"
            fi
        fi

        # Fallback to system PATH
        for b in mariadbd mariadb mariadb-install-db mysqladmin mariadb-admin; do
            if [ ! -f "${OUTPUT_DIR}/bin/${b}" ] && command -v "${b}" >/dev/null 2>&1; then
                cp "$(command -v "${b}")" "${OUTPUT_DIR}/bin/"
            fi
        done
        chmod +x "${OUTPUT_DIR}/bin"/* 2>/dev/null || true
        ;;

    *)
        echo "Error: Unknown target ${TARGET}" >&2
        exit 1
        ;;
esac

echo "==> Stripped MariaDB prepared successfully in ${OUTPUT_DIR}."
