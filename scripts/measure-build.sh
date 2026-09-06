#!/usr/bin/env bash
set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PACKAGE_PATH="${PACKAGE_PATH:-}"
JRE_ROOT="${JRE_HOME:-${JAVA_HOME:-}}"
REPORT_PATH="${REPORT_PATH:-}"

if [ -n "$REPORT_PATH" ]; then
  exec > >(tee "$REPORT_PATH") 2>&1
fi

bytes() {
  if [ -e "$1" ]; then
    stat -Lc '%s' "$1"
  else
    echo 0
  fi
}

human() {
  if [ -e "$1" ]; then
    du -shL "$1" | awk '{print $1}'
  else
    echo "missing"
  fi
}

display_path() {
  if [ ! -e "$1" ]; then
    printf '%s' "$1"
  elif [[ "$1" == "$PROJECT_ROOT"/* ]]; then
    realpath --relative-to="$PROJECT_ROOT" "$1"
  else
    printf '%s' "$1"
  fi
}

report_file() {
  local path="$1"
  if [ -e "$path" ]; then
    printf '%-36s %12s %s\n' "$(display_path "$path")" "$(human "$path")" "$(bytes "$path") bytes"
  else
    printf '%-36s %12s\n' "$(display_path "$path")" "missing"
  fi
}

echo "Bakeneko experimental benchmark"
echo "commit: $(git -C "$PROJECT_ROOT" rev-parse HEAD)"
echo "branch: $(git -C "$PROJECT_ROOT" branch --show-current)"
echo "kernel: $(uname -srmo)"
echo
echo "Artifacts"
report_file "$PROJECT_ROOT/target/release/bakeneko"
report_file "$PROJECT_ROOT/target/release/bakeneko-solver"
report_file "$PROJECT_ROOT/daemon/build/libs/bakeneko-daemon.jar"

if [ -n "$JRE_ROOT" ] && [ -d "$JRE_ROOT" ]; then
  echo
  echo "Java runtime"
  report_file "$JRE_ROOT/bin/java"
  report_file "$JRE_ROOT/bin/keytool"
  report_file "$JRE_ROOT/lib/security/cacerts"
  printf '%-36s %12s\n' "$(display_path "$JRE_ROOT")" "$(human "$JRE_ROOT")"
fi

if [ -n "$PACKAGE_PATH" ]; then
  echo
  echo "Portable package"
  report_file "$PACKAGE_PATH"
  if [ -s "$PACKAGE_PATH" ]; then
    echo "package contents:"
    tar -tzf "$PACKAGE_PATH" | awk '
      /\/app\/jre\/$/ { next }
      { count++; if ($0 ~ /\/app\/jre\//) jre++; else other++ }
      END { printf "  entries=%d jre_entries=%d other_entries=%d\n", count, jre, other }'
  fi
fi

echo
echo "Release profile"
awk '/^\[profile.release\]/{show=1; next} /^\[/{show=0} show{print}' "$PROJECT_ROOT/Cargo.toml"
