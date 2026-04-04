class Forge < Formula
  desc "Unified software modeling DSL — architecture diagrams, docs, and lint from code"
  homepage "https://github.com/grahambrookss/forge"
  version "2026.04.04"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/grahambrookss/forge/releases/download/v2026.04.04/forge-macos-aarch64"
      sha256 "PLACEHOLDER_SHA256"
    else
      url "https://github.com/grahambrookss/forge/releases/download/v2026.04.04/forge-macos-x86_64"
      sha256 "PLACEHOLDER_SHA256"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/grahambrookss/forge/releases/download/v2026.04.04/forge-linux-aarch64"
      sha256 "PLACEHOLDER_SHA256"
    else
      url "https://github.com/grahambrookss/forge/releases/download/v2026.04.04/forge-linux-x86_64"
      sha256 "PLACEHOLDER_SHA256"
    end
  end

  def install
    binary = Dir["*"].first
    bin.install binary => "forge"
  end

  test do
    assert_match "forge", shell_output("#{bin}/forge --version")
  end
end
