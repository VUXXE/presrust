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
mkdir -p "${ROOT_DIR}/prestashop/var/cache"
mkdir -p "${ROOT_DIR}/prestashop/var/logs"

# 1. PHP Runtime
if [ -f "${RUNTIME_DIR}/php/php-fpm" ] || [ -f "${RUNTIME_DIR}/php/php-cgi" ]; then
    echo "--> Existing PHP binary found in ${RUNTIME_DIR}/php."
elif command -v php-cgi >/dev/null 2>&1; then
    echo "--> Linking system php-cgi..."
    ln -sf "$(command -v php-cgi)" "${RUNTIME_DIR}/php/php-cgi"
elif command -v php >/dev/null 2>&1; then
    echo "--> Creating php-cgi wrapper around system php..."
    cat << 'EOF' > "${RUNTIME_DIR}/php/php-cgi"
#!/usr/bin/env bash
exec php -S 127.0.0.1:9000 "$@"
EOF
    chmod +x "${RUNTIME_DIR}/php/php-cgi"
else
    echo "--> Creating mock php-cgi stub..."
    cat << EOF > "${RUNTIME_DIR}/php/php-cgi"
#!/usr/bin/env bash
LOG_FILE="${ROOT_DIR}/logs/php_errors.log"
echo "[\$(date '+%H:%M:%S')] [PHP-CGI] Mock PHP-CGI service started on 127.0.0.1:9000" >> "\${LOG_FILE}"
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
    cat << EOF > "${RUNTIME_DIR}/nginx/sbin/nginx"
#!/usr/bin/env bash
LOG_FILE="${ROOT_DIR}/logs/nginx_access.log"
echo "[\$(date '+%H:%M:%S')] [Nginx] Mock Nginx worker started (port 8080)" >> "\${LOG_FILE}"
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
    cat << EOF > "${RUNTIME_DIR}/mariadb/bin/mariadbd"
#!/usr/bin/env bash
LOG_FILE="${ROOT_DIR}/logs/mariadb_error.log"
echo "[\$(date '+%H:%M:%S')] [MariaDB] Server socket created 127.0.0.1:3306" >> "\${LOG_FILE}"
echo "[\$(date '+%H:%M:%S')] [MariaDB] mysqld ready for connections" >> "\${LOG_FILE}"
while true; do sleep 1; done
EOF
    chmod +x "${RUNTIME_DIR}/mariadb/bin/mariadbd"
fi

# Additional MariaDB helpers
if command -v mariadb-install-db >/dev/null 2>&1; then
    ln -sf "$(command -v mariadb-install-db)" "${RUNTIME_DIR}/mariadb/bin/mariadb-install-db"
elif command -v mysql_install_db >/dev/null 2>&1; then
    ln -sf "$(command -v mysql_install_db)" "${RUNTIME_DIR}/mariadb/bin/mariadb-install-db"
fi

if command -v mariadb >/dev/null 2>&1; then
    ln -sf "$(command -v mariadb)" "${RUNTIME_DIR}/mariadb/bin/mariadb"
elif command -v mysql >/dev/null 2>&1; then
    ln -sf "$(command -v mysql)" "${RUNTIME_DIR}/mariadb/bin/mariadb"
fi

if [ -d "/usr/share/mariadb" ]; then
    ln -sfn "/usr/share/mariadb" "${RUNTIME_DIR}/mariadb/share/mariadb"
elif [ -d "/usr/share/mysql" ]; then
    ln -sfn "/usr/share/mysql" "${RUNTIME_DIR}/mariadb/share/mysql"
fi

echo "==> Local dev environment ready! You can now test the launcher with 'cargo run' in launcher/."
