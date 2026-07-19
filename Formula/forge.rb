class Forge < Formula
  desc "Unified software modeling DSL — architecture diagrams, docs, and lint from code"
  homepage "https://github.com/grahambrooks/forge"
  version "2026.7.1"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/grahambrooks/forge/releases/download/v2026.7.1/forge-v2026.7.1-aarch64-apple-darwin.tar.gz"
      sha256 "c44f817d7ce59852a1b3366a39171e2337f4bf6217a8046e08eb186a5813cbbd"
    end
    on_intel do
      odie "Intel Mac binaries are not provided. Run `cargo install --git https://github.com/grahambrooks/forge --locked` to build from source."
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/grahambrooks/forge/releases/download/v2026.7.1/forge-v2026.7.1-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "f0d3a6018a3258440a38f3a042493254462172e765b6d801f9845a79d9c886c7"
    end
    on_intel do
      url "https://github.com/grahambrooks/forge/releases/download/v2026.7.1/forge-v2026.7.1-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "f4c241075424c0e2e632dbb0b20457c4f611badc5a4fd647f3948a5fb128cc9b"
    end
  end

  def install
    bin.install "forge"
  end

  test do
    assert_path_exists bin/"forge"
  end
end
