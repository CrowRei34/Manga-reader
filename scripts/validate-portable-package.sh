#!/usr/bin/env bash
set -euo pipefail

PACKAGE="${1:?Uso: $0 paquete.tar.gz}"
if [ ! -s "$PACKAGE" ]; then
  echo "ERROR: no existe el paquete: $PACKAGE" >&2
  exit 1
fi

PACKAGE_DIR="$(cd "$(dirname "$PACKAGE")" && pwd)"
PACKAGE_FILE="$(basename "$PACKAGE")"
CHECKSUM_FILE="$PACKAGE_DIR/$PACKAGE_FILE.sha256"
PERMISSIONS_REPORT="$(mktemp "${TMPDIR:-/tmp}/bakeneko-package-permissions.XXXXXX")"

if [ -f "$CHECKSUM_FILE" ]; then
  (cd "$PACKAGE_DIR" && sha256sum -c "$PACKAGE_FILE.sha256")
fi

# No regular file or directory in a release should be world-writable.
if tar -tvzf "$PACKAGE" | awk 'substr($1, 1, 1) != "l" && substr($1, 9, 1) == "w" { bad = 1; print } END { exit bad }' \
    >"$PERMISSIONS_REPORT"; then
  :
else
  echo "ERROR: el archivo contiene entradas world-writable:" >&2
  cat "$PERMISSIONS_REPORT" >&2
  rm -f "$PERMISSIONS_REPORT"
  exit 1
fi
rm -f "$PERMISSIONS_REPORT"

TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/bakeneko-package-check.XXXXXX")"
trap 'rm -rf "$TMP_DIR"' EXIT
tar -xzf "$PACKAGE" -C "$TMP_DIR"

TOP="$(find "$TMP_DIR" -mindepth 1 -maxdepth 1 -type d -name 'Bakeneko-Portable-*' -print -quit)"
if [ -z "$TOP" ]; then
  echo "ERROR: falta el directorio raíz del paquete." >&2
  exit 1
fi

test -x "$TOP/bakeneko"
test -x "$TOP/app/bakeneko"
test -f "$TOP/app/bakeneko-daemon.jar"
test -x "$TOP/app/jre/bin/java"
test -f "$TOP/app/jre/lib/security/cacerts"
test -f "$TOP/app/assets/bakeneko.svg"

for forbidden in javac javadoc jlink jpackage jmod; do
  if [ -e "$TOP/app/jre/bin/$forbidden" ]; then
    echo "ERROR: el runtime contiene una herramienta de JDK: $forbidden" >&2
    exit 1
  fi
done
for forbidden in jmods include man src.zip; do
  if [ -e "$TOP/app/jre/$forbidden" ]; then
    echo "ERROR: el runtime contiene payload de desarrollo: $forbidden" >&2
    exit 1
  fi
done

if find "$TOP" -type f -perm -002 -print -quit | grep -q .; then
  echo "ERROR: el paquete extraído contiene archivos world-writable." >&2
  exit 1
fi

"$TOP/app/jre/bin/java" -version 2>&1 | sed -n '1p'

# Smoke-test the JVM daemon without requiring a graphical compositor. The
# escape hatch is useful when running this checker in a sandbox that blocks
# Unix-domain sockets; CI deliberately leaves it unset.
if [ "${SKIP_DAEMON_SMOKE:-0}" = "1" ]; then
  echo "Paquete portable válido (smoke test omitido): $PACKAGE_FILE"
  exit 0
fi

mkdir -p "$TMP_DIR/home" "$TMP_DIR/runtime"
chmod 700 "$TMP_DIR/runtime"
XDG_RUNTIME_DIR="$TMP_DIR/runtime" HOME="$TMP_DIR/home" \
  timeout 12s "$TOP/app/jre/bin/java" -jar "$TOP/app/bakeneko-daemon.jar" \
  >"$TMP_DIR/daemon.log" 2>&1 &
DAEMON_PID=$!
SOCKET="$TMP_DIR/runtime/bakeneko/daemon.sock"
READY=0
for _ in $(seq 1 120); do
  if [ -S "$SOCKET" ]; then
    READY=1
    break
  fi
  if ! kill -0 "$DAEMON_PID" 2>/dev/null; then
    break
  fi
  sleep 0.1
done

if [ "$READY" -ne 1 ]; then
  cat "$TMP_DIR/daemon.log" >&2
  kill -TERM "$DAEMON_PID" 2>/dev/null || true
  wait "$DAEMON_PID" 2>/dev/null || true
  echo "ERROR: el daemon no creó su socket Unix." >&2
  exit 1
fi

kill -TERM "$DAEMON_PID" 2>/dev/null || true
wait "$DAEMON_PID" 2>/dev/null || true
echo "Paquete portable válido: $PACKAGE_FILE"
