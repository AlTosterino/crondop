class Crondrop < Formula
  desc "Desktop eye-drop reminder app with a CLI-first workflow"
  homepage "https://github.com/AlTosterino/crondop"
  license "MIT"
  version "0.1.0"

  on_macos do
    url "https://github.com/AlTosterino/crondop/releases/download/v0.1.0/crondrop-macos-aarch64.tar.gz"
    sha256 "ba1aee4b29f70505c7547bc0cd0750974d9df420ae85243b67a227f2cb65f609"
  end

  on_linux do
    url "https://github.com/AlTosterino/crondop/releases/download/v0.1.0/crondrop-linux-x86_64.tar.gz"
    sha256 "da782241557f3c76bae469043f76e680df6c3ac781b82f381462d11322861444"
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
