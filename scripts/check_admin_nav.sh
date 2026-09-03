#!/usr/bin/env bash
# Every sidebar href must have a Route; every Route (except *) must have a nav href.
set -euo pipefail
repo_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo_root"

APP="admin/src/App.tsx"
NAV="admin/src/lib/nav.ts"

if [[ ! -f "$APP" || ! -f "$NAV" ]]; then
  echo "✓ Admin nav check skipped (no App.tsx / nav.ts)"
  exit 0
fi

fail=0
hrefs="$(grep -oE 'href: "[^"]+"' "$NAV" | sed 's/href: "//;s/"//' || true)"

while IFS= read -r href; do
  [[ -z "$href" ]] && continue
  if [[ "$href" == "/" ]]; then
    if ! grep -qE '<Route[[:space:]]+index' "$APP"; then
      echo "✗ nav href / has no <Route index>"
      fail=1
    fi
  else
    path="${href#/}"
    if ! grep -qE "path=\"${path}\"" "$APP"; then
      echo "✗ nav href $href has no <Route path=\"$path\">"
      fail=1
    fi
  fi
done <<< "$hrefs"

if grep -qE '<Route[[:space:]]+index' "$APP"; then
  if ! grep -qx '/' <<<"$hrefs"; then
    echo "✗ <Route index> has no nav href /"
    fail=1
  fi
fi

while IFS= read -r path; do
  [[ -z "$path" || "$path" == "*" ]] && continue
  if ! grep -qx "/$path" <<<"$hrefs"; then
    echo "✗ <Route path=\"$path\"> has no nav href /$path"
    fail=1
  fi
done < <(grep -oE 'path="[^"]+"' "$APP" | sed 's/path="//;s/"//')

if [[ "$fail" -eq 0 ]]; then
  echo "✓ Admin nav: hrefs and routes match"
fi
exit "$fail"
