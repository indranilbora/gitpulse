class Agentpulse < Formula
  desc "Agent-first terminal hub for monitoring local Git repositories"
  homepage "https://github.com/indranilbora/agentpulse"
  license "MIT"
  url "https://github.com/indranilbora/agentpulse.git",
      tag:      "v0.1.0",
      revision: "d27dbf2948ce0b7cc095aef693cdd6030e0b2f98"
  head "https://github.com/indranilbora/agentpulse.git", branch: "master"

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
