#!/usr/bin/env bash
set -euo pipefail

VERSION="${1:-0.1.0}"
BUILD_DIR="${CPACK_BUILD_DIR:-build/cpack}"
JAVA_MODULES="${BAKENEKO_JAVA_MODULES:-}"

if [ -z "${SOURCE_DATE_EPOCH:-}" ]; then
  if command -v git >/dev/null 2>&1 && git rev-parse --verify HEAD >/dev/null 2>&1; then
    SOURCE_DATE_EPOCH="$(git log -1 --format=%ct)"
  else
    SOURCE_DATE_EPOCH="$(date +%s)"
  fi
  export SOURCE_DATE_EPOCH
fi

if [ "${SKIP_BUILD:-0}" != "1" ]; then
  cargo build --release --locked
  (cd daemon && ./gradlew test shadowJar --no-daemon)
fi

JDK_HOME="${JDK_HOME:-${JRE_HOME:-}}"
if [ -n "$JDK_HOME" ] && [ -d "$JDK_HOME" ]; then
  JDK_SRC="$(readlink -f "$JDK_HOME")"
elif JAVA_BIN="$(command -v java 2>/dev/null)" && [ -n "$JAVA_BIN" ]; then
  JDK_SRC="$(dirname "$(dirname "$(readlink -f "$JAVA_BIN")")")"
else
  echo "ERROR: configura JDK_HOME con un JDK 21 válido." >&2
  exit 1
fi
if [ ! -x "$JDK_SRC/bin/java" ] || [ ! -x "$JDK_SRC/bin/jlink" ] || [ ! -d "$JDK_SRC/jmods" ]; then
  echo "ERROR: se necesita un JDK completo con jlink y jmods: $JDK_SRC" >&2
  exit 1
fi
JAVA_VERSION="$($JDK_SRC/bin/java -version 2>&1 | sed -n '1p')"
JAVA_MAJOR="$(printf '%s\n' "$JAVA_VERSION" | sed -n 's/.*version "\([0-9][0-9]*\).*/\1/p')"
if [ -z "$JAVA_MAJOR" ] || [ "$JAVA_MAJOR" -lt 21 ]; then
  echo "ERROR: se requiere Java 21 o superior; se encontró: $JAVA_VERSION" >&2
  exit 1
fi

DAEMON_JAR="${PWD}/daemon/build/libs/bakeneko-daemon.jar"
if [ -z "$JAVA_MODULES" ]; then
  JAVA_MODULES="$($JDK_SRC/bin/jdeps --ignore-missing-deps --print-module-deps "$DAEMON_JAR" 2>/dev/null || true)"
  JAVA_MODULES="${JAVA_MODULES:-java.base,java.instrument,java.logging,java.net.http,java.scripting,java.xml,jdk.dynalink,jdk.unsupported}"
  for required_module in jdk.crypto.ec jdk.localedata; do
    case ",$JAVA_MODULES," in
      *",$required_module,"*) ;;
      *) JAVA_MODULES="$JAVA_MODULES,$required_module" ;;
    esac
  done
fi

RUNTIME_DIR="$BUILD_DIR/runtime"
rm -rf "$RUNTIME_DIR"
echo "Construyendo runtime Java reducido con módulos: $JAVA_MODULES"
"$JDK_SRC/bin/jlink" \
  --module-path "$JDK_SRC/jmods" \
  --add-modules "$JAVA_MODULES" \
  --strip-debug \
  --no-header-files \
  --no-man-pages \
  --compress=2 \
  --output "$RUNTIME_DIR"

if [ ! -x "$RUNTIME_DIR/bin/java" ] || [ ! -f "$RUNTIME_DIR/lib/security/cacerts" ]; then
  echo "ERROR: jlink no generó un runtime Java utilizable." >&2
  exit 1
fi
echo "Runtime Java reducido: $(du -sh "$RUNTIME_DIR" | awk '{print $1}')"

cmake -S . -B "$BUILD_DIR" \
  -DBAKENEKO_VERSION="$VERSION" \
  -DBAKENEKO_JRE_HOME="$RUNTIME_DIR"
cmake --build "$BUILD_DIR"
cpack --config "$BUILD_DIR/CPackConfig.cmake" -G TGZ -B "$PWD/dist"

PACKAGE="$PWD/dist/Bakeneko-Portable-v${VERSION}-Linux-x86_64.tar.gz"
test -s "$PACKAGE"
(cd "$PWD/dist" && sha256sum "$(basename "$PACKAGE")") > "$PACKAGE.sha256"
echo "$PACKAGE"
