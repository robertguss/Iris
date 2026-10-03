#!/usr/bin/env bash
# Pinned local mail capture for invitation delivery (S19). The one owner of the
# capture's configuration: loopback only, cleared environment, no relay.
#   mailpit.sh install
#   mailpit.sh run <smtp-port> <ui-port> <database>
set -euo pipefail

VERSION="v1.31.2"
SUM_DARWIN_ARM64="d0180f1fc6e47908e80657dcf2aa1a3186c70ec45ae20a18243a3a46f618d3c6"
SUM_LINUX_AMD64="397a14cad03ae34d7c5f13215fd24971fed5ce48bf4237554829b0a10d4306ad"

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
dev="$here/../.dev"
bin="$dev/bin/mailpit"
self="bash apps/reference/scripts/mailpit.sh"

installed() {
  [ -x "$bin" ] && [ "$("$bin" version --no-release-check 2>/dev/null | head -n 1 | awk '{print $2}')" = "$VERSION" ]
}

sha256() {
  if command -v sha256sum >/dev/null 2>&1; then sha256sum "$1" | awk '{print $1}'; else shasum -a 256 "$1" | awk '{print $1}'; fi
}

install() {
  if installed; then
    echo "mailpit $VERSION already installed at $bin"
    return 0
  fi
  local platform want
  case "$(uname -s)-$(uname -m)" in
    Darwin-arm64) platform="darwin-arm64"; want="$SUM_DARWIN_ARM64" ;;
    Linux-x86_64) platform="linux-amd64"; want="$SUM_LINUX_AMD64" ;;
    *) echo "mailpit install: unsupported platform $(uname -s)-$(uname -m)" >&2; return 1 ;;
  esac
  local tmp
  tmp="$(mktemp -d)"
  trap 'rm -rf "$tmp"' RETURN
  curl -fsSL --proto '=https' --tlsv1.2 -o "$tmp/mailpit.tar.gz" \
    "https://github.com/axllent/mailpit/releases/download/$VERSION/mailpit-$platform.tar.gz"
  if [ "$(sha256 "$tmp/mailpit.tar.gz")" != "$want" ]; then
    echo "mailpit install: checksum mismatch; nothing installed" >&2
    return 1
  fi
  tar -xzf "$tmp/mailpit.tar.gz" -C "$tmp" mailpit
  mkdir -p "$dev/bin"
  mv "$tmp/mailpit" "$bin.new"
  chmod 755 "$bin.new"
  mv "$bin.new" "$bin"
  installed || { echo "mailpit install: installed binary is not $VERSION" >&2; return 1; }
  echo "installed mailpit $VERSION at $bin"
}

run() {
  [ $# -eq 3 ] || { echo "usage: $self run <smtp-port> <ui-port> <database>" >&2; return 2; }
  installed || { echo "mailpit $VERSION is not installed; run: $self install" >&2; return 1; }
  # env -i drops every inherited MP_* variable, which could add a POP3 listener,
  # chaos or a relay.
  exec env -i PATH=/usr/bin:/bin HOME="$HOME" "$bin" \
    --smtp "127.0.0.1:$1" --listen "127.0.0.1:$2" --database "$3" \
    --max 500 --max-age 24h --disable-version-check --smtp-disable-rdns
}

case "${1:-}" in
  install) shift; install "$@" ;;
  run) shift; run "$@" ;;
  *) echo "usage: $self install | run <smtp-port> <ui-port> <database>" >&2; exit 2 ;;
esac
