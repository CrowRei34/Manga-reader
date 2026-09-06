#!/usr/bin/env bash
set -euo pipefail

VERSION="${1:-0.1.0}"
BUILD_DIR="${CPACK_BUILD_DIR:-build/cpack}"
DIST_DIR="${CPACK_DIST_DIR:-dist}"

if [ "${SKIP_BUILD:-0}" != "1" ]; then
  cargo build --release --locked
  (cd daemon && ./gradlew test shadowJar --no-daemon)
fi

if [ -n "${JRE_HOME:-}" ] && [ -d "$JRE_HOME" ]; then
  JRE_SRC="$JRE_HOME"
elif JAVA_BIN="$(command -v java 2>/dev/null)" && [ -n "$JAVA_BIN" ]; then
  JRE_SRC="$(dirname "$(dirname "$(readlink -f "$JAVA_BIN")")")"
else
  echo "ERROR: configura JRE_HOME con un JRE/JDK 21 válido." >&2
  exit 1
fi
if [ ! -x "$JRE_SRC/bin/java" ] || [ ! -x "$JRE_SRC/bin/keytool" ]; then
  echo "ERROR: JRE inválido o incompleto: $JRE_SRC" >&2
  exit 1
fi
JAVA_VERSION="$($JRE_SRC/bin/java -version 2>&1 | sed -n '1p')"
case "$JAVA_VERSION" in
  *'version "21.'*|*'version "22.'*|*'version "23.'*|*'version "24.'*|*'version "25.'*) ;;
  *) echo "ERROR: se requiere Java 21 o superior; se encontró: $JAVA_VERSION" >&2; exit 1 ;;
esac
echo "Incluyendo JRE: $JRE_SRC ($JAVA_VERSION)"

JRE_FOR_PACKAGE="$JRE_SRC"
if [ "${BAKENEKO_JLINK:-0}" = "1" ]; then
  JLINK_BIN="$JRE_SRC/bin/jlink"
  if [ ! -x "$JLINK_BIN" ]; then
    echo "ERROR: BAKENEKO_JLINK=1 requiere un JDK con jlink: $JRE_SRC" >&2
    exit 1
  fi

  JRE_FOR_PACKAGE="$BUILD_DIR/jre-jlink"
  rm -rf "$JRE_FOR_PACKAGE"
  mkdir -p "$BUILD_DIR"

  # Esta lista cubre Kotlin, coroutines, OkHttp/Nashorn, TLS y SQLite. jdeps
  # añade módulos detectados en el JAR cuando está disponible; la lista base
  # evita que la reflexión de Nashorn deje un runtime incompleto.
  JLINK_MODULES="${BAKENEKO_JLINK_MODULES:-java.base,java.compiler,java.datatransfer,java.desktop,java.instrument,java.logging,java.management,java.management.rmi,java.naming,java.net.http,java.prefs,java.rmi,java.scripting,java.security.jgss,java.security.sasl,java.sql,java.transaction.xa,java.xml,jdk.crypto.ec,jdk.crypto.cryptoki,jdk.unsupported,jdk.zipfs}"
  if [ -x "$JRE_SRC/bin/jdeps" ]; then
    DETECTED_MODULES="$($JRE_SRC/bin/jdeps --ignore-missing-deps --print-module-deps \
      --multi-release 21 "$PWD/daemon/build/libs/bakeneko-daemon.jar" 2>/dev/null || true)"
    if [ -n "$DETECTED_MODULES" ]; then
      JLINK_MODULES="$(printf '%s,%s\n' "$JLINK_MODULES" "$DETECTED_MODULES" \
        | tr ',' '\n' | sed '/^$/d' | sort -u | paste -sd, -)"
    fi
  fi
  echo "Creando runtime jlink con módulos: $JLINK_MODULES"
  "$JLINK_BIN" \
    --add-modules "$JLINK_MODULES" \
    --strip-debug \
    --no-man-pages \
    --no-header-files \
    --compress=2 \
    --output "$JRE_FOR_PACKAGE"

  # Algunas distribuciones generan cacerts como enlace o no lo incluyen en
  # imágenes mínimas; copiar el almacén real mantiene HTTPS funcional.
  if [ ! -e "$JRE_FOR_PACKAGE/lib/security/cacerts" ]; then
    mkdir -p "$JRE_FOR_PACKAGE/lib/security"
    cp -L "$JRE_SRC/lib/security/cacerts" "$JRE_FOR_PACKAGE/lib/security/cacerts"
  fi
  echo "Runtime jlink: $(du -sh "$JRE_FOR_PACKAGE" | awk '{print $1}')"
fi

cmake -S . -B "$BUILD_DIR" \
  -DBAKENEKO_VERSION="$VERSION" \
  -DBAKENEKO_JRE_HOME="$JRE_FOR_PACKAGE"
cmake --build "$BUILD_DIR"
cpack --config "$BUILD_DIR/CPackConfig.cmake" -G TGZ -B "$PWD/$DIST_DIR"

PACKAGE="$PWD/$DIST_DIR/Bakeneko-Portable-v${VERSION}-Linux-x86_64.tar.gz"
test -s "$PACKAGE"
(cd "$PWD/$DIST_DIR" && sha256sum "$(basename "$PACKAGE")") > "$PACKAGE.sha256"
echo "$PACKAGE"
