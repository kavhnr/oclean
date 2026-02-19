class Oclean < Formula
  desc "Process-cleanup wrapper for opencode sessions"
  homepage "https://github.com/kavhnr/oclean"
  version "0.1.0"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/kavhnr/oclean/releases/download/v0.1.0/oclean-v0.1.0-aarch64-apple-darwin.tar.gz"
      sha256 "083c6c8bd086a3e474235e2ad1a5a0204a73251df5dfa8fb8009c23ca9b66a00"
    else
      odie "x86_64 macOS package is not published yet"
    end
  end

  on_linux do
    url "https://github.com/kavhnr/oclean/releases/download/v0.1.0/oclean-v0.1.0-x86_64-unknown-linux-gnu.tar.gz"
    sha256 "495f121f96ec93a9d9de661c701ed6807da36d575dc135b9929ffcb59fabe456"
  end

  def install
    bin.install "oclean"
  end

  test do
    output = shell_output("#{bin}/oclean --version")
    assert_match(/\d+\.\d+\.\d+/, output)
  end
end
