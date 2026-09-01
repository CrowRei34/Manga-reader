#!/usr/bin/env bash
set -euo pipefail

VERSION="${1:-0.1.0}"
BUILD_DIR="${CPACK_BUILD_DIR:-build/cpack}"

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

cmake -S . -B "$BUILD_DIR" \
  -DBAKENEKO_VERSION="$VERSION" \
  -DBAKENEKO_JRE_HOME="$JRE_SRC"
cmake --build "$BUILD_DIR"
cpack --config "$BUILD_DIR/CPackConfig.cmake" -G TGZ -B "$PWD/dist"

PACKAGE="$PWD/dist/Bakeneko-Portable-v${VERSION}-Linux-x86_64.tar.gz"
test -s "$PACKAGE"
(cd "$PWD/dist" && sha256sum "$(basename "$PACKAGE")") > "$PACKAGE.sha256"
echo "$PACKAGE"
