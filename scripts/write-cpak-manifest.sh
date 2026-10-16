#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 2 ]]; then
  echo "Uso: $0 VERSION OCI_IMAGE_DIGEST" >&2
  exit 2
fi

version="$1"
image_digest="$2"

[[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]
[[ "$image_digest" =~ ^ghcr\.io/[a-z0-9._/-]+@sha256:[0-9a-f]{64}$ ]]

python3 - "$version" "$image_digest" <<'PY'
import json
import pathlib
import sys

version, image = sys.argv[1:]
manifest = {
    "$schema": "https://raw.githubusercontent.com/Containerpak/cpak/v2/schema/manifest-v3.json",
    "manifest_version": "3.0",
    "name": "Bakeneko",
    "description": "Lector de manga para Linux x86_64.",
    "version": version,
    "image": image,
    "binaries": ["/usr/bin/bakeneko"],
    "desktop_entries": ["/usr/share/applications/com.crowrei34.Bakeneko.desktop"],
    "form_factors": ["desktop"],
    "dependencies": [],
    "addons": [],
    "idle_time": 0,
    "override": {
        "socketWayland": True,
        "displayX11": True,
        "clipboard": {"hostToApp": True, "appToHost": True},
        "deviceDri": True,
        "deviceShm": True,
        "filesystem": [{"path": "home", "access": "read-write"}],
        "network": True,
    },
}
pathlib.Path("cpak.json").write_text(
    json.dumps(manifest, indent=2, ensure_ascii=False) + "\n",
    encoding="utf-8",
)
PY

echo "Manifest cpak.json generado para $image_digest"
