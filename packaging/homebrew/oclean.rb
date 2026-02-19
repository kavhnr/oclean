class Oclean < Formula
  desc "Process-cleanup wrapper for opencode sessions"
  homepage "https://github.com/kavhnr/oclean"
  version "0.1.1"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/kavhnr/oclean/releases/download/v0.1.1/oclean-v0.1.1-aarch64-apple-darwin.tar.gz"
      sha256 "f91bd71d6e8c0dc0d3228c03fac0873a3db74fbc16bf81d29c8fc8d006db7521"
    else
      odie "x86_64 macOS package is not published yet"
    end
  end

  on_linux do
    url "https://github.com/kavhnr/oclean/releases/download/v0.1.1/oclean-v0.1.1-x86_64-unknown-linux-gnu.tar.gz"
    sha256 "65388338d840483ee6502d89cae6f8b082a36440574630d5c0a0abcf736d00b9"
  end

  def install
    bin.install "oclean"
  end

  test do
    output = shell_output("#{bin}/oclean --version")
    assert_match(/\d+\.\d+\.\d+/, output)
  end
end
