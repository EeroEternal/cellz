#!/usr/bin/env bash
# Copy the Admin kit (vocabulary) into another repo. Does not overwrite pages or nav.
set -euo pipefail
if [[ $# -lt 1 ]]; then
  echo "Usage: $0 <target-repo-root>"
  exit 1
fi
src="$(cd "$(dirname "$0")/.." && pwd)"
dst="$(cd "$1" && pwd)"

copy() {
  local rel="$1"
  mkdir -p "$(dirname "$dst/$rel")"
  if [[ -d "$src/$rel" ]]; then
    mkdir -p "$dst/$rel"
    rsync -a --delete --exclude='*.test.ts' --exclude='*.test.tsx' "$src/$rel/" "$dst/$rel/"
  else
    mkdir -p "$(dirname "$dst/$rel")"
    cp "$src/$rel" "$dst/$rel"
  fi
  echo "→ $rel"
}

copy admin/src/components/ui
copy admin/src/components/layout
copy admin/src/common
copy admin/src/index.css
copy admin/src/lib/utils.ts
copy admin/src/lib/i18n.ts
copy admin/tailwind.config.js
copy admin/postcss.config.js
copy docs/design.md
copy docs/design
copy .agents/skills/admin-ui-change
copy scripts/check_ui_stack.sh
copy scripts/check_admin_nav.sh

echo "Kit synced into $dst (pages/ and nav.ts left untouched)."
