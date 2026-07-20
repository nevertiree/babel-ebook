#!/usr/bin/env bash
# Detect potential API keys and tokens accidentally committed to the repo.
# Invoked by pre-commit (always_run: true, pass_filenames: false) from the
# repository root, so `git grep` scans every tracked file by default.
set -euo pipefail

PATTERNS="sk-[a-zA-Z0-9]{20,}|sk-(ant|live|test|proj)-[a-zA-Z0-9]{20,}|Bearer\s+[a-zA-Z0-9_-]{20,}|api[_-]?key\s*=\s*[\"\']?[a-zA-Z0-9_-]{20,}"

if git grep -EIn "$PATTERNS"; then
  echo "ERROR: Potential API key or token detected in source files." >&2
  echo "Remove the secret and use environment variables or a keyring instead." >&2
  exit 1
fi
