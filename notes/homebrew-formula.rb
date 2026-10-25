class Imgsmith < Formula
  desc "Convert, resize, crop and thumbnail images from one static binary"
  homepage "https://github.com/marceanlabs/imgsmith"
  version "0.1.1"
  license "Apache-2.0"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/marceanlabs/imgsmith/releases/download/v0.1.1/imgsmith-0.1.1-aarch64-apple-darwin.tar.gz"
      sha256 "344ff836c65596078db498168e6fb90e822eacb80e76df71beabe831b6b5287e"
    else
      url "https://github.com/marceanlabs/imgsmith/releases/download/v0.1.1/imgsmith-0.1.1-x86_64-apple-darwin.tar.gz"
      sha256 "366cdfee5b4096a92142d81b6168e19991ead14a74908d8001f364417bae6e43"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/marceanlabs/imgsmith/releases/download/v0.1.1/imgsmith-0.1.1-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "fee6d15397d665fea116c950e1edd437c865ba935b373936927fb608a261838e"
    else
      url "https://github.com/marceanlabs/imgsmith/releases/download/v0.1.1/imgsmith-0.1.1-x86_64-unknown-linux-musl.tar.gz"
      sha256 "a316c9385293414c73e2c7db5726e6532aa41f1d96f3c98959bf1a2f8c44a52d"
    end
  end

  def install
    bin.install "imgsmith"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/imgsmith --version")
  end
end
