#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

case "${1:-run}" in
build)
    podman build -t beans-dev "$SCRIPT_DIR"
    ;;
run)
    CREDS="$(security find-generic-password -s "Claude Code-credentials" -a "$(whoami)" -w 2>/dev/null || true)"
    if [[ -z "$CREDS" ]]; then
        echo "Error: Could not read Claude Code credentials from Keychain." >&2
        exit 1
    fi
    OAUTH_ACCOUNT="$(jq '.oauthAccount' ~/.claude.json)"
    podman volume exists beans-home 2>/dev/null || podman volume create beans-home
    exec podman run --rm -it --name beans-dev \
        -v beans-home:/home/dev:Z \
        -v "$SCRIPT_DIR:/home/dev/beans" \
        -w /home/dev/beans \
        --userns=keep-id \
        -e CLAUDE_OAUTH="$CREDS" -e CLAUDE_OAUTH_ACCOUNT="$OAUTH_ACCOUNT" \
        beans-dev
    ;;
*)
    echo "Usage: $0 [build|run]" >&2
    exit 1
    ;;
esac
