class Forge < Formula
  desc "Unified software modeling DSL — architecture diagrams, docs, and lint from code"
  homepage "https://github.com/grahambrooks/forge"
  version "2026.9.7"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/grahambrooks/forge/releases/download/v2026.9.7/forge-v2026.9.7-aarch64-apple-darwin.tar.gz"
      sha256 "bfe22aa51b1c6ceddc0ae79e52e91463e75ed392acc500d75c05363495e66934"
    end
    on_intel do
      odie "Intel Mac binaries are not provided. Run `cargo install --git https://github.com/grahambrooks/forge forge-dsl --locked` to build from source."
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/grahambrooks/forge/releases/download/v2026.9.7/forge-v2026.9.7-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "057cbf6c2269ebfda2318a9f931557441098a8b88cd577745251c3b6180c6335"
    end
    on_intel do
      url "https://github.com/grahambrooks/forge/releases/download/v2026.9.7/forge-v2026.9.7-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "e50b6839bb441078779818af26cbb7d7d7e50baa7d94a032feeea29f8820984d"
    end
  end

  def install
    bin.install "forge"
  end

  test do
    assert_path_exists bin/"forge"
  end
end
