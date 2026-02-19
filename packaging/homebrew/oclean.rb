class Oclean < Formula
  desc "Process-cleanup wrapper for opencode sessions"
  homepage "https://github.com/kavhnr/oclean"
  version "0.1.0"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/kavhnr/oclean/releases/download/v0.1.0/oclean-v0.1.0-aarch64-apple-darwin.tar.gz"
      sha256 "<SHA256_AARCH64_APPLE_DARWIN>"
    else
      url "https://github.com/kavhnr/oclean/releases/download/v0.1.0/oclean-v0.1.0-x86_64-apple-darwin.tar.gz"
      sha256 "<SHA256_X86_64_APPLE_DARWIN>"
    end
  end

  on_linux do
    url "https://github.com/kavhnr/oclean/releases/download/v0.1.0/oclean-v0.1.0-x86_64-unknown-linux-gnu.tar.gz"
    sha256 "<SHA256_X86_64_UNKNOWN_LINUX_GNU>"
  end

  def install
    bin.install "oclean"
  end

  test do
    output = shell_output("#{bin}/oclean --version")
    assert_match(/\d+\.\d+\.\d+/, output)
  end
end
