#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 2 ]]; then
  echo "Usage: $0 <version> <changelog>" >&2
  exit 1
fi

version="${1#v}"
changelog="$2"

if [[ ! -f "$changelog" ]]; then
  echo "Changelog not found: $changelog" >&2
  exit 1
fi

awk -v version="$version" '
  BEGIN { in_section = 0; found = 0 }
  $0 ~ "^## \\[" version "\\]" {
    in_section = 1
    found = 1
    next
  }
  in_section && $0 ~ "^## \\[" {
    exit
  }
  in_section {
    print
  }
  END {
    if (!found) {
      exit 2
    }
  }
' "$changelog"
