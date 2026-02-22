class Gitpulse < Formula
  desc "Monitor many local Git repositories in one terminal UI"
  homepage "https://github.com/indranilbora/gitpulse"
  url "https://github.com/indranilbora/gitpulse/archive/refs/tags/v0.1.0.tar.gz"
  sha256 "6b55399c568d78c9c97e13cfeee71ae11bba63df0ad2dc3e9cc2a1670f43dde4"
  version "0.1.0"
  license "MIT"
  head "https://github.com/indranilbora/gitpulse.git", branch: "main"

  depends_on "git"
  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args(path: ".")
  end

  test do
    output = shell_output("#{bin}/gitpulse --help")
    assert_match "Monitor all your git repos from one TUI", output
  end
end
