#!/usr/bin/env bash
set -euo pipefail

version="${1:-dev}"
os_name="$(uname -s | tr '[:upper:]' '[:lower:]')"
arch_name="$(uname -m)"
case "${os_name}" in
  darwin) target_os="darwin" ;;
  linux) target_os="linux" ;;
  *) target_os="${os_name}" ;;
esac
case "${arch_name}" in
  x86_64|amd64) target_arch="amd64" ;;
  arm64|aarch64) target_arch="arm64" ;;
  *) target_arch="${arch_name}" ;;
esac

out_dir="dist"
artifact="codux-service-${version}-${target_os}-${target_arch}"
binary="codux-service"
mkdir -p "${out_dir}/${artifact}"

cargo build --release -p codux-server
cp "target/release/codux-server" "${out_dir}/${artifact}/${binary}"
cp "apps/server/deploy/config.toml" "${out_dir}/${artifact}/config.toml"

(
  cd "${out_dir}"
  tar -czf "${artifact}.tar.gz" "${artifact}"
  shasum -a 256 "${artifact}.tar.gz" > "${artifact}.tar.gz.sha256"
)
