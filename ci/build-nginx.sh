#!/usr/bin/env bash
set -euo pipefail

TARGET="${1:-linux-x86_64}"
OUTPUT_DIR="${2:-build/runtime/${TARGET}/nginx}"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"

if [ -f "${ROOT_DIR}/versions.env" ]; then
    # shellcheck disable=SC1091
    source "${ROOT_DIR}/versions.env"
fi

NGINX_VERSION="${NGINX_VERSION:-1.26.3}"

echo "==> Building/Fetching Nginx ${NGINX_VERSION} for ${TARGET}..."
mkdir -p "${OUTPUT_DIR}"

case "${TARGET}" in
    windows-x86_64)
        echo "--> Downloading official Windows Nginx ${NGINX_VERSION}..."
        WIN_NGINX_URL="https://nginx.org/download/nginx-${NGINX_VERSION}.zip"
        TEMP_ZIP="$(mktemp --suffix=.zip 2>/dev/null || mktemp).zip"
        curl -fsSL -o "${TEMP_ZIP}" "${WIN_NGINX_URL}"
        
        EXTRACT_TMP="$(mktemp -d)"
        unzip -q "${TEMP_ZIP}" -d "${EXTRACT_TMP}"
        # Copy contents of nginx-1.26.3 to OUTPUT_DIR
        cp -r "${EXTRACT_TMP}"/nginx-*/* "${OUTPUT_DIR}/"
        rm -rf "${EXTRACT_TMP}" "${TEMP_ZIP}"
        echo "--> Windows Nginx prepared in ${OUTPUT_DIR}"
        ;;

    linux-x86_64|macos-arm64|macos-x86_64)
        echo "--> Building lean Nginx from source..."
        BUILD_TMP="$(mktemp -d)"
        pushd "${BUILD_TMP}" > /dev/null

        SRC_URL="https://nginx.org/download/nginx-${NGINX_VERSION}.tar.gz"
        curl -fsSL -o nginx.tar.gz "${SRC_URL}"
        tar -xzf nginx.tar.gz
        cd "nginx-${NGINX_VERSION}"

        # Configure lean Nginx
        ./configure \
            --prefix="" \
            --sbin-path="sbin/nginx" \
            --conf-path="conf/nginx.conf" \
            --pid-path="logs/nginx.pid" \
            --error-log-path="logs/error.log" \
            --http-log-path="logs/access.log" \
            --with-http_ssl_module \
            --with-http_v2_module \
            --with-http_realip_module \
            --without-http_uwsgi_module \
            --without-http_scgi_module \
            --without-mail_pop3_module \
            --without-mail_imap_module \
            --without-mail_smtp_module

        make -j"$(nproc 2>/dev/null || sysctl -n hw.ncpu 2>/dev/null || echo 2)"

        # Collect binary and conf
        mkdir -p "${OUTPUT_DIR}/sbin" "${OUTPUT_DIR}/conf"
        cp objs/nginx "${OUTPUT_DIR}/sbin/nginx"
        cp conf/mime.types "${OUTPUT_DIR}/conf/mime.types"
        cp conf/fastcgi.conf "${OUTPUT_DIR}/conf/fastcgi.conf"
        chmod +x "${OUTPUT_DIR}/sbin/nginx"

        popd > /dev/null
        rm -rf "${BUILD_TMP}"
        echo "--> Nginx built successfully into ${OUTPUT_DIR}"
        ;;

    *)
        echo "Error: Unknown target ${TARGET}" >&2
        exit 1
        ;;
esac

echo "==> Nginx preparation complete for ${TARGET}."
