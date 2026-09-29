#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"

TARGET="$(uname -s | tr '[:upper:]' '[:lower:]')-$(uname -m)"
case "${TARGET}" in
    linux-x86_64) TARGET="linux-x86_64" ;;
    darwin-arm64) TARGET="macos-arm64" ;;
    darwin-x86_64) TARGET="macos-x86_64" ;;
    msys*|mingw*|cygwin*) TARGET="windows-x86_64" ;;
    *) echo "Defaulting target to linux-x86_64"; TARGET="linux-x86_64" ;;
esac

RUNTIME_DIR="${ROOT_DIR}/runtime/${TARGET}"
echo "==> Setting up local dev environment for ${TARGET} in ${RUNTIME_DIR}..."

mkdir -p "${RUNTIME_DIR}/nginx/sbin"
mkdir -p "${RUNTIME_DIR}/nginx/conf"
mkdir -p "${RUNTIME_DIR}/php"
mkdir -p "${RUNTIME_DIR}/mariadb/bin"
mkdir -p "${RUNTIME_DIR}/mariadb/share"
mkdir -p "${ROOT_DIR}/logs"
mkdir -p "${ROOT_DIR}/tmp/sessions"
mkdir -p "${ROOT_DIR}/tmp/uploads"
mkdir -p "${ROOT_DIR}/data/mariadb"

# 1. PHP Mock / System Link
if command -v php-cgi >/dev/null 2>&1; then
    echo "--> Linking system php-cgi..."
    ln -sf "$(command -v php-cgi)" "${RUNTIME_DIR}/php/php-cgi"
elif command -v php >/dev/null 2>&1; then
    echo "--> Creating php-cgi wrapper around system php..."
    cat << 'EOF' > "${RUNTIME_DIR}/php/php-cgi"
#!/usr/bin/env bash
# Wrapper around php built-in server or fastcgi if available
exec php -S 127.0.0.1:9000 "$@"
EOF
    chmod +x "${RUNTIME_DIR}/php/php-cgi"
else
    echo "--> Creating mock php-cgi stub..."
    cat << 'EOF' > "${RUNTIME_DIR}/php/php-cgi"
#!/usr/bin/env bash
echo "[$(date '+%H:%M:%S')] [PHP-CGI] Mock PHP-CGI service started on 127.0.0.1:9000"
while true; do sleep 1; done
EOF
    chmod +x "${RUNTIME_DIR}/php/php-cgi"
fi

# 2. Nginx Mock / System Link
if command -v nginx >/dev/null 2>&1; then
    echo "--> Linking system nginx..."
    ln -sf "$(command -v nginx)" "${RUNTIME_DIR}/nginx/sbin/nginx"
else
    echo "--> Creating mock nginx stub..."
    cat << 'EOF' > "${RUNTIME_DIR}/nginx/sbin/nginx"
#!/usr/bin/env bash
echo "[$(date '+%H:%M:%S')] [Nginx] Mock Nginx worker started (port 8080)"
while true; do sleep 1; done
EOF
    chmod +x "${RUNTIME_DIR}/nginx/sbin/nginx"
fi

# 3. MariaDB Mock / System Link
if command -v mariadbd >/dev/null 2>&1; then
    echo "--> Linking system mariadbd..."
    ln -sf "$(command -v mariadbd)" "${RUNTIME_DIR}/mariadb/bin/mariadbd"
elif command -v mysqld >/dev/null 2>&1; then
    echo "--> Linking system mysqld..."
    ln -sf "$(command -v mysqld)" "${RUNTIME_DIR}/mariadb/bin/mariadbd"
else
    echo "--> Creating mock mariadbd stub..."
    cat << 'EOF' > "${RUNTIME_DIR}/mariadb/bin/mariadbd"
#!/usr/bin/env bash
echo "[$(date '+%H:%M:%S')] [MariaDB] Server socket created 127.0.0.1:3306"
echo "[$(date '+%H:%M:%S')] [MariaDB] mysqld ready for connections"
while true; do sleep 1; done
EOF
    chmod +x "${RUNTIME_DIR}/mariadb/bin/mariadbd"
fi

echo "==> Local dev environment ready! You can now test the launcher with 'cargo run' in launcher/."
