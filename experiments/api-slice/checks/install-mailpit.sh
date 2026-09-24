#!/usr/bin/env bash
set -euo pipefail
destination="${1:-$HOME/.local/bin/mailpit}"
if [[ -x "$destination" ]] && "$destination" version | grep -q 'v1.31.2'; then exit 0; fi
[[ "$(uname -s)-$(uname -m)" == Linux-x86_64 ]] || { echo 'This pinned fixture installer supports Linux x86_64 only.' >&2; exit 1; }
temp="$(mktemp -d)"
trap 'rm -rf "$temp"' EXIT
curl -fsSL --retry 3 https://github.com/axllent/mailpit/releases/download/v1.31.2/mailpit-linux-amd64.tar.gz -o "$temp/mailpit.tar.gz"
echo "397a14cad03ae34d7c5f13215fd24971fed5ce48bf4237554829b0a10d4306ad  $temp/mailpit.tar.gz" | sha256sum -c -
tar -xzf "$temp/mailpit.tar.gz" -C "$temp" mailpit
mkdir -p "$(dirname "$destination")"
install -m 0755 "$temp/mailpit" "$destination"
