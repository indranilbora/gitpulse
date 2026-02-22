class Gitpulse < Formula
  desc "Monitor many local Git repositories in one terminal UI"
  homepage "https://github.com/indranilbora/gitpulse"
  url "https://static.crates.io/crates/gitpulse/gitpulse-0.1.0.crate"
  sha256 "61622bc28c6d2d673edfdacb1d78186122d9dc2ad27794dccb9883be6faeacd5"
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
