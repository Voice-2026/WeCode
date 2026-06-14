#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 ]]; then
  echo "usage: verify-ios-ffi-symbols.sh <Codux.ipa|Runner.app>" >&2
  exit 2
fi

input_path="$1"
if [[ ! -e "$input_path" ]]; then
  echo "Input not found: $input_path" >&2
  exit 2
fi

work_dir="$(mktemp -d)"
trap 'rm -rf "$work_dir"' EXIT

if [[ -d "$input_path" && "$input_path" == *.app ]]; then
  app_path="$input_path"
else
  unzip -q "$input_path" -d "$work_dir"
  app_path="$(find "$work_dir/Payload" -maxdepth 1 -type d -name '*.app' | head -n 1)"
fi
if [[ -z "$app_path" ]]; then
  echo "Input does not contain an app bundle" >&2
  exit 1
fi

required_symbols=(
  codux_protocol_version
  codux_terminal_text_input_json
  codux_controller_transport_connect_json
  codux_remote_runtime_model_new
  codux_remote_runtime_model_apply_project_list_json
  codux_output_router_new
)

mach_o_files=()
while IFS= read -r file_path; do
  if file "$file_path" | grep -q 'Mach-O'; then
    mach_o_files+=("$file_path")
  fi
done < <(find "$app_path" -type f)

if (( ${#mach_o_files[@]} == 0 )); then
  echo "No Mach-O files found in IPA" >&2
  exit 1
fi

missing=()
for symbol in "${required_symbols[@]}"; do
  found=false
  for mach_o in "${mach_o_files[@]}"; do
    symbol_table="$(nm -gU "$mach_o" 2>/dev/null || true)"
    if grep "_${symbol}$" <<< "$symbol_table" >/dev/null; then
      found=true
      break
    fi
  done
  if [[ "$found" != "true" ]]; then
    missing+=("$symbol")
  fi
done

if (( ${#missing[@]} > 0 )); then
  echo "Missing iOS FFI symbols in final IPA:" >&2
  printf '  - %s\n' "${missing[@]}" >&2
  echo "The iOS app uses DynamicLibrary.process(), so release archives must keep global FFI symbols." >&2
  exit 1
fi

echo "Verified iOS FFI symbols in $(basename "$input_path")"
