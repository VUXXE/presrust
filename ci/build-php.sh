#!/usr/bin/env bash
set -euo pipefail

TARGET="${1:-linux-x86_64}"
OUTPUT_DIR="${2:-build/runtime/${TARGET}/php}"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"

# Load versions
if [ -f "${ROOT_DIR}/versions.env" ]; then
    # shellcheck disable=SC1091
    source "${ROOT_DIR}/versions.env"
fi

PHP_VERSION="${PHP_VERSION:-8.4.5}"
PHP_SERIES="${PHP_VERSION%.*}" # e.g. 8.4
SPC_VERSION="${STATIC_PHP_CLI_VERSION:-2.5.0}"

echo "==> Building/Fetching PHP ${PHP_VERSION} for ${TARGET}..."
mkdir -p "${OUTPUT_DIR}"

REQUIRED_EXTENSIONS="pdo,pdo_mysql,mysqli,curl,gd,intl,zip,fileinfo,mbstring,openssl,iconv,simplexml,dom,xml,xmlwriter,ctype,tokenizer,session,filter,hash,opcache,sodium,bcmath"

case "${TARGET}" in
    windows-x86_64)
        echo "--> Downloading official Windows PHP ${PHP_VERSION} NTS x64..."
        # Windows official PHP distribution from windows.php.net
        WIN_PHP_URL="https://windows.php.net/downloads/releases/php-${PHP_VERSION}-nts-Win32-vs17-x64.zip"
        TEMP_ZIP="$(mktemp --suffix=.zip 2>/dev/null || mktemp).zip"
        
        # Fallback if specific patch version isn't yet at release URL: try fetching latest in series or archives
        if ! curl -fsSL -o "${TEMP_ZIP}" "${WIN_PHP_URL}"; then
            echo "--> Notice: ${WIN_PHP_URL} failed, checking archives..."
            WIN_PHP_ARCHIVE="https://windows.php.net/downloads/releases/archives/php-${PHP_VERSION}-nts-Win32-vs17-x64.zip"
            curl -fsSL -o "${TEMP_ZIP}" "${WIN_PHP_ARCHIVE}"
        fi
        
        unzip -q -o "${TEMP_ZIP}" -d "${OUTPUT_DIR}"
        rm -f "${TEMP_ZIP}"
        echo "--> Windows PHP extracted to ${OUTPUT_DIR}"
        ;;

    linux-x86_64|macos-arm64|macos-x86_64)
        echo "--> Building static php-cgi via static-php-cli (${SPC_VERSION})..."
        SPC_BIN="${ROOT_DIR}/build/tools/spc"
        mkdir -p "${ROOT_DIR}/build/tools"

        if [ ! -f "${SPC_BIN}" ]; then
            echo "--> Downloading static-php-cli release binary..."
            SPC_OS="linux"
            SPC_ARCH="x86_64"
            if [[ "${TARGET}" == macos* ]]; then
                SPC_OS="macos"
                if [[ "${TARGET}" == "macos-arm64" ]]; then
                    SPC_ARCH="aarch64"
                fi
            fi
            
            SPC_URL="https://github.com/crazywhalecc/static-php-cli/releases/download/${SPC_VERSION}/spc-${SPC_OS}-${SPC_ARCH}.tar.gz"
            TEMP_TGZ="$(mktemp --suffix=.tar.gz 2>/dev/null || mktemp).tar.gz"
            curl -fsSL -o "${TEMP_TGZ}" "${SPC_URL}"
            tar -xzf "${TEMP_TGZ}" -C "${ROOT_DIR}/build/tools"
            chmod +x "${SPC_BIN}"
            rm -f "${TEMP_TGZ}"
        fi

        echo "--> Fetching source dependencies via spc..."
        # NOTE: spc download hits many upstreams; transient 403/56 errors
        # happen on shared CI runners. Retry before giving up.
        SPC_ATTEMPT=1
        until "${SPC_BIN}" download --with-php="${PHP_SERIES}" --for-extensions="${REQUIRED_EXTENSIONS}"; do
            if [ "${SPC_ATTEMPT}" -ge 3 ]; then
                echo "Error: spc download failed after 3 attempts" >&2
                exit 1
            fi
            echo "--> spc download failed (attempt ${SPC_ATTEMPT}), retrying in 15s..."
            SPC_ATTEMPT=$((SPC_ATTEMPT + 1))
            sleep 15
        done

        echo "--> Compiling php-fpm and php-cli statically..."
        # NOTE: spc 2.x has no --build-cgi SAPI flag. php-fpm speaks
        # FastCGI exactly like php-cgi, and the launcher already prefers
        # php-fpm (see PhpService::find_binary / is_fpm branch).
        if [[ "${TARGET}" == "linux-x86_64" ]]; then
            # spc defaults to a musl target on glibc distros, demanding a
            # musl-cross-make toolchain at /usr/local/musl. Our bundle
            # already links glibc (nginx is compiled on-runner, MariaDB
            # ships official glibc binaries), so a musl PHP adds no
            # portability — build against glibc instead.
            export SPC_LIBC=glibc
        fi
        "${SPC_BIN}" build "${REQUIRED_EXTENSIONS}" --build-fpm --build-cli

        # spc outputs to buildroot/bin/
        if [ -f "buildroot/bin/php-cgi" ]; then
            cp "buildroot/bin/php-cgi" "${OUTPUT_DIR}/php-cgi"
            chmod +x "${OUTPUT_DIR}/php-cgi"
        fi
        if [ -f "buildroot/bin/php-fpm" ]; then
            cp "buildroot/bin/php-fpm" "${OUTPUT_DIR}/php-fpm"
            chmod +x "${OUTPUT_DIR}/php-fpm"
        fi
        if [ -f "buildroot/bin/php" ]; then
            cp "buildroot/bin/php" "${OUTPUT_DIR}/php"
            chmod +x "${OUTPUT_DIR}/php"
        fi
        echo "--> Static PHP built successfully into ${OUTPUT_DIR}"
        ;;

    *)
        echo "Error: Unknown target ${TARGET}" >&2
        exit 1
        ;;
esac

echo "==> PHP preparation complete for ${TARGET}."
