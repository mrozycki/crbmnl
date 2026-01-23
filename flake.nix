{
  description = "crbmnl - Rust implementation of custom TRMNL OS";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };


  outputs = { self, nixpkgs, rust-overlay, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system: 
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        rust = pkgs.rust-bin.stable."1.92.0".default.override {
          extensions = [ "rust-src" ];
        };
        rustPlatform = pkgs.makeRustPlatform {
          cargo = rust;
          rustc = rust;
        };
      in
      {
        packages = rec {
          crbmnl = rustPlatform.buildRustPackage rec {
            pname = "crbmnl";
            version = "0.1.0";

            src = ./.;
            cargoLock = {
              lockFile = ./Cargo.lock;
            };
          };
          default = crbmnl;
        };
      });
}
