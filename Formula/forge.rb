class Forge < Formula
  desc "Unified software modeling DSL — architecture diagrams, docs, and lint from code"
  homepage "https://github.com/grahambrooks/forge"
  version "2026.04.05"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://api.github.com/repos/grahambrooks/forge/releases/assets/ASSET_ID_MACOS_ARM",
          headers: ["Authorization: token #{ENV.fetch("HOMEBREW_GITHUB_API_TOKEN")}", "Accept: application/octet-stream"]
      sha256 "PLACEHOLDER_SHA256"
    else
      url "https://api.github.com/repos/grahambrooks/forge/releases/assets/ASSET_ID_MACOS_X86",
          headers: ["Authorization: token #{ENV.fetch("HOMEBREW_GITHUB_API_TOKEN")}", "Accept: application/octet-stream"]
      sha256 "PLACEHOLDER_SHA256"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://api.github.com/repos/grahambrooks/forge/releases/assets/ASSET_ID_LINUX_ARM",
          headers: ["Authorization: token #{ENV.fetch("HOMEBREW_GITHUB_API_TOKEN")}", "Accept: application/octet-stream"]
      sha256 "PLACEHOLDER_SHA256"
    else
      url "https://api.github.com/repos/grahambrooks/forge/releases/assets/ASSET_ID_LINUX_X86",
          headers: ["Authorization: token #{ENV.fetch("HOMEBREW_GITHUB_API_TOKEN")}", "Accept: application/octet-stream"]
      sha256 "PLACEHOLDER_SHA256"
    end
  end

  def install
    binary = Dir["*"].first
    bin.install binary => "forge"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/forge --version")
  end
end
