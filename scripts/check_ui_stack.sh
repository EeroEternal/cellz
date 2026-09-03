#!/usr/bin/env bash
# Fail closed on Greenfield HTML: product UI must live in admin/ or frontend/.
set -euo pipefail
repo_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo_root"

fail=0

has_kit=0
[[ -d admin/src ]] && has_kit=1
[[ -d frontend/src ]] && has_kit=1

# Standalone HTML next to the backend is the usual agent escape hatch.
while IFS= read -r f; do
  echo "✗ Greenfield HTML: $f (use admin/ or frontend/ React kit)"
  fail=1
done < <(find . -maxdepth 2 -name '*.html' \
  ! -path './.git/*' ! -path './target/*' ! -path './node_modules/*' \
  ! -path './admin/*' ! -path './frontend/*' ! -path './www/*' \
  ! -path './docs/*' 2>/dev/null)

if [[ "$has_kit" -eq 0 ]]; then
  echo "✗ No UI kit (missing admin/src and frontend/src)."
  echo "  Do not generate pages. Scaffold the kit first, or refuse the UI task."
  fail=1
fi

if [[ "$fail" -eq 0 ]]; then
  echo "✓ UI stack check: kit present, no stray HTML"
fi
exit "$fail"
