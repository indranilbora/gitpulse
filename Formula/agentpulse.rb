class Agentpulse < Formula
  desc "Agent-first terminal hub for monitoring local Git repositories"
  homepage "https://github.com/indranilbora/gitpulse"
  license "MIT"
  head "https://github.com/indranilbora/gitpulse.git", branch: "master"

  depends_on "git"
  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args(path: ".")
  end

  test do
    output = shell_output("#{bin}/agentpulse --help")
    assert_match "Agent-first terminal hub", output
  end
end
