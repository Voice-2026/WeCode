#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 4 ]]; then
  echo "Usage: $0 <version> <output> <english-changelog> <chinese-changelog>" >&2
  exit 1
fi

version="${1#v}"
output="$2"
english_changelog="$3"
chinese_changelog="$4"
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

english_notes="$(mktemp)"
chinese_notes="$(mktemp)"
trap 'rm -f "$english_notes" "$chinese_notes"' EXIT

bash "$script_dir/extract-release-notes.sh" "$version" "$english_changelog" > "$english_notes"
bash "$script_dir/extract-release-notes.sh" "$version" "$chinese_changelog" > "$chinese_notes"

if [[ ! -s "$english_notes" ]]; then
  echo "Missing or empty English mobile release notes for $version in $english_changelog" >&2
  exit 2
fi

if [[ ! -s "$chinese_notes" ]]; then
  echo "Missing or empty Chinese mobile release notes for $version in $chinese_changelog" >&2
  exit 2
fi

{
  echo "# WeCode Mobile v$version"
  echo
  echo "## English"
  cat "$english_notes"
  echo
  echo "## 简体中文"
  cat "$chinese_notes"
} > "$output"
