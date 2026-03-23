#!/usr/bin/env bash
set -euo pipefail

if [[ -n "${CLAUDE_OAUTH:-}" ]]; then
    echo "$CLAUDE_OAUTH" > /home/dev/.claude/.credentials.json
fi

if [[ -n "${CLAUDE_OAUTH_ACCOUNT:-}" ]]; then
    tmp=$(jq --argjson acct "$CLAUDE_OAUTH_ACCOUNT" '.oauthAccount = $acct' /home/dev/.claude.json)
    echo "$tmp" > /home/dev/.claude.json
fi

unset CLAUDE_OAUTH CLAUDE_OAUTH_ACCOUNT
exec "${@:-bash}"
