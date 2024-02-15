{
  description = "Home page";

  inputs = {
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    nixpkgs.url = "nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    wgsl.url = "github:wgsl-analyzer/wgsl-analyzer/";
  };
  outputs = { self, nixpkgs, flake-utils, fenix, wgsl }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        linker_pkgs = with pkgs; [ mold clang ];

        fast_rust_toolchain = fenix.packages.${system}.complete.withComponents [
          "rustc"
          "cargo"
          "clippy"
          "rustfmt"
          "rust-src"
          "rust-std"
          "rustc-codegen-cranelift-preview"
          "rust-analyzer"
        ];
        rust_toolchain = fenix.packages.${system}.stable.withComponents [
          "rustc"
          "cargo"
          "clippy"
          "rustfmt"
          "rust-src"
          "rust-std"
          "rust-analyzer"
        ];
        fast_run_command = pkgs.writeShellApplication {
          name = "frun";
          text = ''
            RUSTFLAGS="-Z threads=10 -C target-cpu=native -Zcodegen-backend=cranelift" cargo run \$@
          '';
        };
        run_command = pkgs.writeShellApplication {
          name = "run";
          text = ''
            cargo run \$@
          '';
        };
      in with pkgs; {
        nixpkgs.overlays = [ fenix.overlays.complete ];
        devShells.default = mkShell rec {

          nativeBuildInputs =
            [ pkg-config cmake pkg-config freetype expat fontconfig mold ];

          buildInputs = [
            sccache
            run_command
            rust_toolchain
            wgsl.packages.${system}.default
            lldb_15
            udev
            alsa-lib
            vulkan-loader
            libxkbcommon
            wayland
            wasm-bindgen-cli
          ] ++ linker_pkgs;
          LD_LIBRARY_PATH =
            nixpkgs.lib.makeLibraryPath (buildInputs ++ nativeBuildInputs);
        };
      });
}

