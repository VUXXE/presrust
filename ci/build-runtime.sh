#!/usr/bin/env bash
set -euo pipefail

TARGET="${1:-linux-x86_64}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "=========================================="
echo " Building Runtime Components for ${TARGET}"
echo "=========================================="

"${SCRIPT_DIR}/build-php.sh" "${TARGET}"
"${SCRIPT_DIR}/build-nginx.sh" "${TARGET}"
"${SCRIPT_DIR}/prepare-mariadb.sh" "${TARGET}"

echo "=========================================="
echo " All runtime components built for ${TARGET}"
echo "=========================================="
