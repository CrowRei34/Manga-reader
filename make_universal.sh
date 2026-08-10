#!/usr/bin/env bash
set -euo pipefail
# Bakeneko-Universal: binario Rust + JAR daemon + JRE. Sin Flutter.
APP_VERSION="${1:-0.1.0}"
APPIMAGE_DIR="AppDir"

rm -rf "$APPIMAGE_DIR" && mkdir -p "$APPIMAGE_DIR/usr/bin" "$APPIMAGE_DIR/usr/lib" "$APPIMAGE_DIR/usr/jre"

cargo build --release

cp target/release/bakeneko "$APPIMAGE_DIR/usr/bin/"

# JAR del daemon: si no existe (dev machine sin el proyecto Java), se advierte
# y se omite — la app igual arranca y reporta el fallo de daemon en la UI.
if [ -f daemon/build/libs/bakeneko-daemon.jar ]; then
  cp daemon/build/libs/bakeneko-daemon.jar "$APPIMAGE_DIR/usr/bin/"
else
  echo "WARN: daemon/build/libs/bakeneko-daemon.jar no existe; omitiendo JAR del daemon." >&2
fi

# JRE: se resuelve el home del java del PATH (o $JRE_HOME si está definido) y
# se copian los CONTENIDOS a usr/jre/ (con "/." ) para que exista
# usr/jre/bin/java, que es lo que espera `resolve_java` vía JAVA_HOME.
if [ -n "${JRE_HOME:-}" ] && [ -d "$JRE_HOME" ]; then
  JRE_SRC="$JRE_HOME"
elif JAVA_BIN="$(command -v java 2>/dev/null)" && [ -n "$JAVA_BIN" ]; then
  JRE_SRC="$(dirname "$(dirname "$(readlink -f "$JAVA_BIN")")")"
else
  JRE_SRC=""
  echo "WARN: no se encontró java; omitiendo JRE bundleado." >&2
fi
if [ -n "${JRE_SRC:-}" ] && [ -d "$JRE_SRC" ]; then
  cp -r "$JRE_SRC/." "$APPIMAGE_DIR/usr/jre/"
else
  echo "WARN: $JRE_SRC no es un directorio JRE válido; omitiendo JRE bundleado." >&2
fi

# AppRun wrapper
cat > "$APPIMAGE_DIR/AppRun" <<'EOF'
#!/bin/sh
SELF=$(readlink -f "$0")
HERE=${SELF%/*}
export JAVA_HOME="$HERE/usr/jre"
exec "$HERE/usr/bin/bakeneko"
EOF
chmod +x "$APPIMAGE_DIR/AppRun"

# Desktop entry + icon: appimagetool aborta sin un .desktop válido.
mkdir -p "$APPIMAGE_DIR/usr/share/icons/hicolor/256x256/apps"
cat > "$APPIMAGE_DIR/bakeneko.desktop" <<'EOF'
[Desktop Entry]
Name=Bakeneko Reader
Comment=Lector de manga (Rust + daemon JVM)
Exec=bakeneko
Icon=bakeneko
Type=Application
Categories=Utility;
EOF
# Icon PNG 256x256, color de acento por defecto (#7c5cbf).
python3 - "$APPIMAGE_DIR/usr/share/icons/hicolor/256x256/apps/bakeneko.png" <<'PYEOF'
import struct, zlib, sys, os
out = sys.argv[1]
os.makedirs(os.path.dirname(out), exist_ok=True)
size = 256
r, g, b = 0x7c, 0x5c, 0xbf
row = b"\x00" + b"".join(struct.pack("BBB", r, g, b) for _ in range(size))
raw = row * size
def chunk(t, data):
    c = t + data
    return struct.pack(">I", len(data)) + c + struct.pack(">I", zlib.crc32(c))
png = b"\x89PNG\r\n\x1a\n"
png += chunk(b"IHDR", struct.pack(">IIBBBBB", size, size, 8, 2, 0, 0, 0))
png += chunk(b"IDAT", zlib.compress(raw))
png += chunk(b"IEND", b"")
with open(out, "wb") as f:
    f.write(png)
print("icon written:", out)
PYEOF
# appimagetool busca el icono junto al .desktop (raíz del AppDir).
cp "$APPIMAGE_DIR/usr/share/icons/hicolor/256x256/apps/bakeneko.png" "$APPIMAGE_DIR/bakeneko.png"

# appimagetool (descargar si falta)
if [ ! -f appimagetool-x86_64.AppImage ]; then
  wget -q https://github.com/AppImage/AppImageKit/releases/download/continuous/appimagetool-x86_64.AppImage
  chmod +x appimagetool-x86_64.AppImage
fi
ARCH=x86_64 ./appimagetool-x86_64.AppImage "$APPIMAGE_DIR" "Bakeneko-Universal-v${APP_VERSION}.AppImage"
