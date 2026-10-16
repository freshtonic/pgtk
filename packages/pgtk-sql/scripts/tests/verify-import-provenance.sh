#!/usr/bin/env bash

set -euo pipefail

readonly repo_root="$(git rev-parse --show-toplevel)"
readonly package=packages/pgtk-sql
readonly verifier="$repo_root/$package/scripts/verify-import-provenance"

"$verifier"

test_dir="$(mktemp -d)"
trap 'rm -rf "$test_dir"' EXIT

git clone --quiet --no-local "$repo_root" "$test_dir/repository"
mkdir -p "$test_dir/repository/$package/scripts"
cp "$verifier" "$test_dir/repository/$package/scripts/verify-import-provenance"
for record in import-provenance.tsv import-provenance.legacy-commit import-provenance.legacy-tree; do
  cp "$repo_root/$package/docs/$record" "$test_dir/repository/$package/docs/$record"
done

(
  cd "$test_dir/repository"
  $package/scripts/verify-import-provenance

  wrong_gitlink="$(git rev-parse HEAD)"
  git update-index --cacheinfo "160000,$wrong_gitlink,vendor/postgres"

  if $package/scripts/verify-import-provenance >/dev/null 2>&1; then
    echo "expected verification to reject a changed PostgreSQL gitlink" >&2
    exit 1
  fi

  git reset --quiet
  sed 's/^# legacy-tree .*/# legacy-tree 0000000000000000000000000000000000000000/' \
    $package/docs/import-provenance.tsv > "$test_dir/changed-provenance.tsv"
  mv "$test_dir/changed-provenance.tsv" $package/docs/import-provenance.tsv

  if $package/scripts/verify-import-provenance >/dev/null 2>&1; then
    echo "expected verification to reject a changed provenance manifest" >&2
    exit 1
  fi
)
