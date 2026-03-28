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
                rustToolchain = pkgs.rust-bin.stable.latest.default.override {
                    extensions = [
                        "rust-src"
                        "rust-analyzer"
                        "clippy"
                        "rustfmt"
                    ];
                };
                nativeBuildInputs = with pkgs; [
                    rustToolchain
                    pkg-config
                ];
                buildInputs = with pkgs; [
                    openssl
                    linux-pam
                    libclang
                    glibc.dev
                ];
            in
            {
                packages.default = pkgs.rustPlatform.buildRustPackage {
                    pname = "sysapi";
                    version = "0.1.0";
                    src = ./.;
                    cargoHash = "sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=";
                    inherit nativeBuildInputs buildInputs;
                    OPENSSL_NO_VENDOR = 1;
                    PKG_CONFIG_PATH = "${pkgs.openssl.dev}/lib/pkgconfig";
                    LIBCLANG_PATH = "${pkgs.libclang.lib}/lib";
                    BINDGEN_EXTRA_CLANG_ARGS = "-I${pkgs.linux-pam}/include -I${pkgs.glibc.dev}/include";
                };
                devShells.default = pkgs.mkShell {
                    inherit nativeBuildInputs buildInputs;
                    PKG_CONFIG_PATH = "${pkgs.openssl.dev}/lib/pkgconfig";
                    OPENSSL_DIR = "${pkgs.openssl.dev}";
                    OPENSSL_LIB_DIR = "${pkgs.openssl.out}/lib";
                    OPENSSL_INCLUDE_DIR = "${pkgs.openssl.dev}/include";
                    LIBCLANG_PATH = "${pkgs.libclang.lib}/lib";
                    BINDGEN_EXTRA_CLANG_ARGS = "-I${pkgs.linux-pam}/include -I${pkgs.glibc.dev}/include";
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
