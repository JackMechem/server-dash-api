{
  description = "sysapi - system stats & command execution REST API in Rust";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      rust-overlay,
      flake-utils,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };

        # Pin to stable. Swap to rust-overlay's nightly if you need it:
        # pkgs.rust-bin.nightly.latest.default
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [
            "rust-src"
            "rust-analyzer"
            "clippy"
            "rustfmt"
          ];
        };

        # Native build inputs required to compile Rust crates with C deps
        nativeBuildInputs = with pkgs; [
          rustToolchain
          pkg-config
        ];

        # Runtime / link-time dependencies
        buildInputs = with pkgs; [
          openssl # required by jsonwebtoken / reqwest
        ];

      in
      {
        devShells.default = pkgs.mkShell {
          inherit nativeBuildInputs buildInputs;

          # Let pkg-config find openssl
          PKG_CONFIG_PATH = "${pkgs.openssl.dev}/lib/pkgconfig";

          # Tells the openssl-sys crate where to look (fallback if pkg-config fails)
          OPENSSL_DIR = "${pkgs.openssl.dev}";
          OPENSSL_LIB_DIR = "${pkgs.openssl.out}/lib";
          OPENSSL_INCLUDE_DIR = "${pkgs.openssl.dev}/include";

          # Points rust-analyzer at the stdlib source
          RUST_SRC_PATH = "${rustToolchain}/lib/rustlib/src/rust/library";

          shellHook = ''
            echo "🦀 sysapi dev shell ready"
            echo "   rustc  $(rustc --version)"
            echo "   cargo  $(cargo --version)"
          '';
        };
      }
    );
}
