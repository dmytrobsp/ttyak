#!/usr/bin/env bash
# Publish docs/wiki/ at a ref to the GitHub wiki repo: export, check links and privacy, mirror, commit, push on --push.
# usage: publish.sh [--ref REF] [--push] [--wiki-dir DIR]     (REF defaults to the latest v* tag; first publish: --ref master)
# Environment: TTYAK_WIKI_URL overrides the wiki remote (tests use a local bare repo).
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ref="" push=0 wiki_dir=""
while [ $# -gt 0 ]; do
  case "$1" in
    --ref) ref="${2:?--ref needs a value}"; shift 2 ;;
    --push) push=1; shift ;;
    --wiki-dir) wiki_dir="${2:?--wiki-dir needs a value}"; shift 2 ;;
    *) echo "usage: publish.sh [--ref REF] [--push] [--wiki-dir DIR]" >&2; exit 2 ;;
  esac
done

repo="$(git rev-parse --show-toplevel)"
cd "$repo"
if [ -z "$ref" ]; then
  ref="$(git describe --tags --abbrev=0 --match 'v*')" || { echo "no v* tag found; use --ref" >&2; exit 1; }
fi
echo "publishing docs/wiki at $ref"
short="$(git rev-parse --short "$ref^{commit}")"
url="${TTYAK_WIKI_URL:-https://github.com/dmytrobsp/ttyak.wiki.git}"

base="${TMPDIR:-/tmp}"
work="$(mktemp -d "$base/ttyak-wiki.XXXXXX")"
case "$work" in "$base"/ttyak-wiki.*) ;; *) echo "unexpected temp dir" >&2; exit 1 ;; esac
cleanup() { if [ -n "${work:-}" ] && [ -d "$work" ] && [ "${work#"$base"/ttyak-wiki.}" != "$work" ]; then rm -rf "${work:?}"; fi; }
trap cleanup EXIT

export_dir="$work/export"
mkdir "$export_dir"
paths=(docs/wiki)
for p in README.md site/index.html docs/images; do
  if git cat-file -e "$ref:$p" 2>/dev/null; then paths+=("$p"); fi
done
git archive "$ref" "${paths[@]}" | tar -x -C "$export_dir"
[ -d "$export_dir/docs/wiki" ] || { echo "no docs/wiki at $ref" >&2; exit 1; }

echo "checking links"
extra=()
[ -f "$export_dir/site/index.html" ] && extra+=("$export_dir/site/index.html")
python3 "$here/check_links.py" --wiki "$export_dir/docs/wiki" --readme "$export_dir/README.md" "${extra[@]:-$export_dir/none.html}" \
  || { echo "refusing: broken links" >&2; exit 1; }

echo "checking privacy"
python3 "$here/../site/check_private.py" "$export_dir/docs/wiki" || { echo "refusing: privacy check failed" >&2; exit 1; }
if [ -d "$export_dir/docs/images" ] && [ -n "$(find "$export_dir/docs/images" -type f \( -name '*.svg' -o -name '*.md' -o -name '*.txt' \) -print -quit)" ]; then
  python3 "$here/../site/check_private.py" "$export_dir/docs/images" || { echo "refusing: privacy check failed on docs/images" >&2; exit 1; }
fi

if [ -z "$wiki_dir" ]; then wiki_dir="$work/wiki"; fi
if [ -d "$wiki_dir/.git" ]; then
  if [ -n "$(git -C "$wiki_dir" ls-remote origin)" ]; then git -C "$wiki_dir" pull -q --ff-only; fi
else
  git clone -q "$url" "$wiki_dir"
fi
if ! git -C "$wiki_dir" rev-parse -q --verify HEAD >/dev/null; then git -C "$wiki_dir" checkout -q -b master; fi

# Mirror: drop what the wiki tracks, copy the export over.
git -C "$wiki_dir" ls-files -z | (cd "$wiki_dir" && xargs -0 -r rm -f --)
tar -C "$export_dir/docs/wiki" -c . | tar -C "$wiki_dir" -x
git -C "$wiki_dir" add -A
if git -C "$wiki_dir" diff --cached --quiet; then
  echo "the wiki already matches docs/wiki at $ref"
  exit 0
fi
git -C "$wiki_dir" commit -q -m "wiki: docs/wiki at $ref ($short)"
git -C "$wiki_dir" show --stat HEAD
git -C "$wiki_dir" show HEAD
if [ "$push" = 1 ]; then
  git -C "$wiki_dir" push -q origin HEAD
  echo "pushed"
else
  echo "not pushed (use --push); the clone is $wiki_dir"
fi
