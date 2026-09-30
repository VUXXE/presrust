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
chmod -R u+w "${OUTPUT_DIR}" 2>/dev/null || true
rm -rf "${OUTPUT_DIR}"
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

        # Bundle Visual C++ Runtime DLLs (vcruntime140.dll, msvcp140.dll, etc.)
        # so PHP runs out-of-the-box on clean Windows systems without requiring external VC Redist installation.
        echo "--> Bundling Visual C++ Runtime DLLs into ${OUTPUT_DIR}..."
        VC_DLLS=(vcruntime140.dll vcruntime140_1.dll msvcp140.dll msvcp140_1.dll msvcp140_2.dll msvcp140_codecvt_ids.dll vcomp140.dll concrt140.dll)
        COPIED_VC=false

        for sys_dir in "/c/Windows/System32" "/c/Windows/SysWOW64" "C:/Windows/System32" "${WINDIR:-}/System32"; do
            if [ -n "${sys_dir}" ] && [ -f "${sys_dir}/vcruntime140.dll" ]; then
                for dll in "${VC_DLLS[@]}"; do
                    if [ -f "${sys_dir}/${dll}" ]; then
                        cp -f "${sys_dir}/${dll}" "${OUTPUT_DIR}/"
                    fi
                done
                COPIED_VC=true
                echo "--> VC++ Runtime DLLs bundled from ${sys_dir}"
                break
            fi
        done

        if [ "${COPIED_VC}" != "true" ]; then
            echo "--> Downloading official Microsoft VC_redist.x64.exe to extract runtime DLLs..."
            VC_REDIST_URL="https://aka.ms/vs/17/release/vc_redist.x64.exe"
            TEMP_REDIST="$(mktemp --suffix=.exe 2>/dev/null || mktemp).exe"
            if curl -fsSL -o "${TEMP_REDIST}" "${VC_REDIST_URL}"; then
                VC_EXTRACT_TMP="$(mktemp -d)"
                if command -v 7z >/dev/null 2>&1; then
                    7z e -y "${TEMP_REDIST}" -o"${VC_EXTRACT_TMP}" "a12" >/dev/null 2>&1 || true
                    if [ -f "${VC_EXTRACT_TMP}/a12" ]; then
                        7z e -y "${VC_EXTRACT_TMP}/a12" -o"${VC_EXTRACT_TMP}/dlls" "*.dll*" >/dev/null 2>&1 || true
                    fi
                elif command -v cabextract >/dev/null 2>&1; then
                    cabextract -q -F "a12" -d "${VC_EXTRACT_TMP}" "${TEMP_REDIST}" 2>/dev/null || true
                    if [ -f "${VC_EXTRACT_TMP}/a12" ]; then
                        mkdir -p "${VC_EXTRACT_TMP}/dlls"
                        cabextract -q -d "${VC_EXTRACT_TMP}/dlls" "${VC_EXTRACT_TMP}/a12" 2>/dev/null || true
                    fi
                fi

                if [ -d "${VC_EXTRACT_TMP}/dlls" ]; then
                    for f in "${VC_EXTRACT_TMP}/dlls"/*; do
                        fname="$(basename "$f")"
                        clean_name="${fname%%_amd64}"
                        clean_name="${clean_name%%_arm64}"
                        if [[ "${clean_name}" == *.dll ]]; then
                            cp -f "$f" "${OUTPUT_DIR}/${clean_name}"
                        fi
                    done
                    echo "--> VC++ Runtime DLLs extracted and bundled into ${OUTPUT_DIR}"
                fi
                rm -rf "${VC_EXTRACT_TMP}" "${TEMP_REDIST}"
            fi
        fi
        ;;

    linux-x86_64|macos-arm64|macos-x86_64)
        SPC_OS="linux"
        SPC_ARCH="x86_64"
        if [[ "${TARGET}" == macos* ]]; then
            SPC_OS="macos"
            if [[ "${TARGET}" == "macos-arm64" ]]; then
                SPC_ARCH="aarch64"
            fi
        fi

        PREBUILT_BASE="https://dl.static-php.dev/static-php-cli/bulk"
        echo "--> Checking for prebuilt static PHP ${PHP_VERSION} for ${TARGET}..."
        
        TEMP_FPM="$(mktemp --suffix=.tar.gz 2>/dev/null || mktemp).tar.gz"
        TEMP_CLI="$(mktemp --suffix=.tar.gz 2>/dev/null || mktemp).tar.gz"
        
        PREBUILT_SUCCESS=false
        if curl -fsSL -o "${TEMP_FPM}" "${PREBUILT_BASE}/php-${PHP_VERSION}-fpm-${SPC_OS}-${SPC_ARCH}.tar.gz" && \
           curl -fsSL -o "${TEMP_CLI}" "${PREBUILT_BASE}/php-${PHP_VERSION}-cli-${SPC_OS}-${SPC_ARCH}.tar.gz"; then
            echo "--> Extracting prebuilt static PHP binaries..."
            tar -xzf "${TEMP_FPM}" -C "${OUTPUT_DIR}"
            tar -xzf "${TEMP_CLI}" -C "${OUTPUT_DIR}"
            chmod +x "${OUTPUT_DIR}/php-fpm" "${OUTPUT_DIR}/php" 2>/dev/null || true
            cp "${OUTPUT_DIR}/php-fpm" "${OUTPUT_DIR}/php-cgi" 2>/dev/null || true
            rm -f "${TEMP_FPM}" "${TEMP_CLI}"
            PREBUILT_SUCCESS=true
            echo "--> Prebuilt static PHP (fpm + cli) prepared successfully in ${OUTPUT_DIR}"
        else
            rm -f "${TEMP_FPM}" "${TEMP_CLI}"
            echo "--> Notice: Prebuilt binaries not available at ${PREBUILT_BASE}, falling back to static-php-cli compile..."
        fi

        if [ "${PREBUILT_SUCCESS}" != "true" ]; then
            echo "--> Building static php via static-php-cli (${SPC_VERSION})..."
            SPC_BIN="${ROOT_DIR}/build/tools/spc"
            mkdir -p "${ROOT_DIR}/build/tools"

            if [ ! -f "${SPC_BIN}" ]; then
                echo "--> Downloading static-php-cli release binary..."
                SPC_URL="https://github.com/crazywhalecc/static-php-cli/releases/download/${SPC_VERSION}/spc-${SPC_OS}-${SPC_ARCH}.tar.gz"
                TEMP_TGZ="$(mktemp --suffix=.tar.gz 2>/dev/null || mktemp).tar.gz"
                curl -fsSL -o "${TEMP_TGZ}" "${SPC_URL}"
                tar -xzf "${TEMP_TGZ}" -C "${ROOT_DIR}/build/tools"
                chmod +x "${SPC_BIN}"
                rm -f "${TEMP_TGZ}"
            fi

            echo "--> Running spc doctor to configure build environment..."
            "${SPC_BIN}" doctor --auto-fix || true

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
            "${SPC_BIN}" build "${REQUIRED_EXTENSIONS}" --build-fpm --build-cli --debug

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
            if [ -f "${OUTPUT_DIR}/php-fpm" ] && [ ! -f "${OUTPUT_DIR}/php-cgi" ]; then
                cp "${OUTPUT_DIR}/php-fpm" "${OUTPUT_DIR}/php-cgi"
            fi
            echo "--> Static PHP built successfully into ${OUTPUT_DIR}"
        fi
        ;;

    *)
        echo "Error: Unknown target ${TARGET}" >&2
        exit 1
        ;;
esac

echo "==> PHP preparation complete for ${TARGET}."
