#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 4 ]]; then
  echo "usage: $0 <owner/repo> <tag> <linux_sha256> <macos_sha256>" >&2
  exit 1
fi

repo="$1"
tag="$2"
linux_sha="$3"
macos_sha="$4"
version="${tag#v}"

cat <<EOF
class Crondrop < Formula
  desc "Desktop eye-drop reminder app with a CLI-first workflow"
  homepage "https://github.com/${repo}"
  license "MIT"
  version "${version}"

  on_macos do
    depends_on arch: :arm64
    url "https://github.com/${repo}/releases/download/${tag}/crondrop-macos-aarch64.tar.gz"
    sha256 "${macos_sha}"
  end

  on_linux do
    url "https://github.com/${repo}/releases/download/${tag}/crondrop-linux-x86_64.tar.gz"
    sha256 "${linux_sha}"
  end

  def install
    bin.install Dir["*/crondrop"].fetch(0)

    readme = Dir["*/README-packaging.md"].first
    doc.install readme if readme

    summary = Dir["*/SUMMARY.MD"].first
    doc.install summary if summary

    plist = Dir["*/com.crondrop.plist"].first
    prefix.install plist if OS.mac? && plist

    desktop = Dir["*/crondrop.desktop"].first
    prefix.install desktop if OS.linux? && desktop
  end

  test do
    assert_match "A friendly CLI-first eye drop reminder", shell_output("#{bin}/crondrop --help")
  end
end
EOF
