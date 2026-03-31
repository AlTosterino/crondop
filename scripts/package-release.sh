#!/usr/bin/env bash
set -euo pipefail

archive_name="${1:?archive name is required}"
binary_path="${2:?binary path is required}"

root_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
dist_dir="${root_dir}/dist"
stage_dir="${dist_dir}/${archive_name}"
doc_source=""

rm -rf "${stage_dir}"
mkdir -p "${stage_dir}"

cp "${root_dir}/${binary_path}" "${stage_dir}/crondrop"
cp "${root_dir}/packaging/README.md" "${stage_dir}/README-packaging.md"

if [[ -f "${root_dir}/SPEC.md" ]]; then
  doc_source="${root_dir}/SPEC.md"
elif [[ -f "${root_dir}/SUMMARY.MD" ]]; then
  doc_source="${root_dir}/SUMMARY.MD"
fi

if [[ -n "${doc_source}" ]]; then
  cp "${doc_source}" "${stage_dir}/$(basename "${doc_source}")"
fi

if [[ -f "${root_dir}/packaging/linux/crondrop.desktop" ]]; then
  cp "${root_dir}/packaging/linux/crondrop.desktop" "${stage_dir}/crondrop.desktop"
fi

if [[ -f "${root_dir}/packaging/macos/com.crondrop.plist" ]]; then
  cp "${root_dir}/packaging/macos/com.crondrop.plist" "${stage_dir}/com.crondrop.plist"
fi

tar -C "${dist_dir}" -czf "${dist_dir}/${archive_name}.tar.gz" "${archive_name}"
