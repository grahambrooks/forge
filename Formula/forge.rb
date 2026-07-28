class Forge < Formula
  desc "Unified software modeling DSL — architecture diagrams, docs, and lint from code"
  homepage "https://github.com/grahambrooks/forge"
  version "2026.7.28"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/grahambrooks/forge/releases/download/v2026.7.28/forge-v2026.7.28-aarch64-apple-darwin.tar.gz"
      sha256 "500d8211e4941f2e79808d6906b4cfceaf748012ed3c08396896f3fc541541c7"
    end
    on_intel do
      odie "Intel Mac binaries are not provided. Run `cargo install --git https://github.com/grahambrooks/forge forge-dsl --locked` to build from source."
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/grahambrooks/forge/releases/download/v2026.7.28/forge-v2026.7.28-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "6ec6da6dde3dd2b35d633e5c0252dc83d6938950dd8200ebf04ea3ff4cca091d"
    end
    on_intel do
      url "https://github.com/grahambrooks/forge/releases/download/v2026.7.28/forge-v2026.7.28-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "7597d5cd936fb20e6eda3c229a91825af80c4be229d6e768611ee9c43cbb20f1"
    end
  end

  def install
    bin.install "forge"
  end

  test do
    assert_path_exists bin/"forge"
  end
end
