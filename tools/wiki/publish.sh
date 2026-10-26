#!/usr/bin/env bash
# Publish docs/wiki/ at a ref to the GitHub wiki repo: export, check links and privacy, mirror, commit, push on --push.
# usage: publish.sh [--ref REF] [--push] [--wiki-dir DIR]     (REF defaults to the latest v* tag; first publish: --ref master)
# --wiki-dir DIR keeps the wiki clone (relative to where you run it; outside the repo). The clone is the script's cache:
# a folder that is already a git clone is reused only when its origin is the wiki remote, and it is reset to the
# wiki's current state first (local commits, edits and untracked files in it are dropped).
# Without --wiki-dir the clone is a temp folder, deleted on exit.
# The wiki commit is made with the main repo's own user.name/user.email (never the global identity); the wiki remote
# is derived from the repo's origin URL (keeps the user@ part the credential helper matches).
# Environment: TTYAK_WIKI_URL overrides the wiki remote (tests use a local bare repo).
set -euo pipefail

# The wiki remote that belongs to a main-repo origin URL: ttyak(.git) -> ttyak.wiki.git, scheme and user@ kept.
wiki_url_from_origin() {
  case "$1" in
    *ttyak.wiki.git) printf '%s\n' "$1" ;;
    *ttyak.git) printf '%s\n' "${1%ttyak.git}ttyak.wiki.git" ;;
    *ttyak) printf '%s\n' "${1}.wiki.git" ;;
    *) return 1 ;;
  esac
}
# Whether DIR (a path that may not exist yet) is the repo, inside it ("inside") or holds it ("holds"): compared by
# inode (os.path.samefile) up the path, so a differently spelled or cased path, or a bind mount, is still caught.
wiki_dir_clash() {
  python3 -c '
import os, sys
repo, d = os.path.abspath(sys.argv[1]), os.path.abspath(sys.argv[2])
def chain(p):
    while True:
        yield p
        up = os.path.dirname(p)
        if up == p:
            return
        p = up
if any(os.path.exists(p) and os.path.samefile(p, repo) for p in chain(d)):
    print("inside")
elif os.path.exists(d) and any(os.path.samefile(p, d) for p in chain(repo)):
    print("holds")
' "$1" "$2"
}
# Sourced by the tests for the helpers only.
if [ "${BASH_SOURCE[0]}" != "$0" ]; then return 0; fi

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

repo="$(cd "$(git rev-parse --show-toplevel)" && pwd -P)"
# --wiki-dir is the caller's path: make it absolute (symlinks resolved) before leaving the caller's folder, and keep it
# apart from the repo, so the mirror's delete-every-tracked-file never runs on the repo itself.
if [ -n "$wiki_dir" ]; then
  wiki_dir="$(python3 -c 'import os, sys; print(os.path.realpath(sys.argv[1]))' "$wiki_dir")"
  case "$(wiki_dir_clash "$repo" "$wiki_dir")" in
    inside) echo "refusing: --wiki-dir $wiki_dir is the repo or inside it" >&2; exit 1 ;;
    holds) echo "refusing: --wiki-dir $wiki_dir holds the repo" >&2; exit 1 ;;
  esac
fi
cd "$repo"
if [ -z "$ref" ]; then
  ref="$(git describe --tags --abbrev=0 --match 'v*')" || { echo "no v* tag found; use --ref" >&2; exit 1; }
fi
echo "publishing docs/wiki at $ref"
short="$(git rev-parse --short "$ref^{commit}")"
id_name="$(git -C "$repo" config --local user.name || true)"
id_email="$(git -C "$repo" config --local user.email || true)"
if [ -z "$id_name" ] || [ -z "$id_email" ]; then
  echo "refusing: the repo has no git identity (set user.name and user.email in $repo)" >&2; exit 1
fi
if [ -n "${TTYAK_WIKI_URL:-}" ]; then
  url="$TTYAK_WIKI_URL"
else
  origin="$(git -C "$repo" config remote.origin.url || true)"
  url="$(wiki_url_from_origin "$origin")" || { echo "refusing: cannot derive the wiki URL from origin '$origin'; set TTYAK_WIKI_URL" >&2; exit 1; }
fi
echo "commit identity: $id_name <$id_email>"
echo "wiki remote: $url"

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
# The repo's own (local) credential settings, so a push from the clone uses the same helper.
copy_credentials() {
  local keys=() k v
  while IFS= read -r -d '' k; do keys+=("${k%%$'\n'*}"); done < <(git -C "$repo" config --local -z --get-regexp '^credential\.' || true)
  [ ${#keys[@]} -gt 0 ] || return 0
  for k in "${keys[@]}"; do git -C "$wiki_dir" config --local --unset-all "$k" 2>/dev/null || true; done
  while IFS= read -r -d '' k; do
    v="${k#*$'\n'}"; [ "$v" = "$k" ] && v=""
    git -C "$wiki_dir" config --local --add "${k%%$'\n'*}" "$v"
  done < <(git -C "$repo" config --local -z --get-regexp '^credential\.' || true)
}
if [ -d "$wiki_dir/.git" ]; then
  have="$(git -C "$wiki_dir" remote get-url origin 2>/dev/null || true)"
  if [ "$have" != "$url" ]; then
    echo "refusing: --wiki-dir $wiki_dir is a git clone whose origin is '$have', not the wiki remote $url" >&2; exit 1
  fi
  copy_credentials
  # A reused clone is only a cache: it becomes exactly the wiki's state (no local commits, edits or leftovers).
  if [ -n "$(git -C "$wiki_dir" ls-remote "$url")" ]; then
    git -C "$wiki_dir" fetch -q origin HEAD
    git -C "$wiki_dir" reset -q --hard FETCH_HEAD
  else
    git -C "$wiki_dir" symbolic-ref HEAD refs/heads/master
    git -C "$wiki_dir" update-ref -d refs/heads/master
  fi
  git -C "$wiki_dir" clean -q -fdx
else
  git clone -q "$url" "$wiki_dir"
  copy_credentials
fi
if ! git -C "$wiki_dir" rev-parse -q --verify HEAD >/dev/null; then git -C "$wiki_dir" symbolic-ref HEAD refs/heads/master; fi
# What the wiki has now (empty when it has no commits yet): a push is due whenever HEAD differs from it.
remote_head="$(git -C "$wiki_dir" ls-remote "$url" HEAD | cut -f1)"

# Mirror: drop what the wiki tracks, copy the export over (tar -m: files get the time of the copy, not the commit's).
git -C "$wiki_dir" ls-files -z | (cd "$wiki_dir" && xargs -0 -r rm -f --)
tar -C "$export_dir/docs/wiki" -c . | tar -m -C "$wiki_dir" -x
# Empty the index so every file is hashed again: a same-size edit with an unchanged mtime must not look unchanged.
git -C "$wiki_dir" read-tree --empty
git -C "$wiki_dir" add -A
if git -C "$wiki_dir" diff --cached --quiet; then
  if [ "$(git -C "$wiki_dir" rev-parse -q --verify HEAD || true)" = "$remote_head" ]; then
    echo "the wiki already matches docs/wiki at $ref"
    exit 0
  fi
  echo "the clone is ahead of the wiki"
else
  GIT_AUTHOR_NAME="$id_name" GIT_AUTHOR_EMAIL="$id_email" GIT_COMMITTER_NAME="$id_name" GIT_COMMITTER_EMAIL="$id_email" \
  git -C "$wiki_dir" -c "user.name=$id_name" -c "user.email=$id_email" commit -q --author="$id_name <$id_email>" -m "wiki: docs/wiki at $ref ($short)"
  git -C "$wiki_dir" show --stat HEAD
  git -C "$wiki_dir" show HEAD
fi
if [ "$push" = 1 ]; then
  git -C "$wiki_dir" push -q "$url" HEAD
  echo "pushed"
else
  if [ "$wiki_dir" = "$work/wiki" ]; then
    echo "not pushed (use --push); the clone $wiki_dir is a temp folder, deleted on exit (--wiki-dir DIR keeps one)"
  else
    echo "not pushed (use --push); the clone is $wiki_dir"
  fi
fi
