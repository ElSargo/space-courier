{
  description = "Home page";

  inputs = {
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    fenixl = {
      url = "github:nix-community/fenix";
    };
    nixpkgs.url = "nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    wgsl.url = "github:wgsl-analyzer/wgsl-analyzer/";
  };
  outputs = {
    self,
    nixpkgs,
    flake-utils,
    fenix,
    fenixl,
    wgsl,
  }:
    flake-utils.lib.eachDefaultSystem (system: let
      pkgs = nixpkgs.legacyPackages.${system};
      linker_pkgs = with pkgs; [mold clang];

      fast_rust_toolchain = fenixl.packages.${system}.latest.withComponents [
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
      # rust_toolchain

      nativeBuildInputs = with pkgs; [pkg-config cmake pkg-config freetype expat fontconfig] ++ linker_pkgs;
      buildInputs = with pkgs; [lldb_15 udev alsa-lib vulkan-loader libxkbcommon wayland wasm-bindgen-cli];
    in
      with pkgs; {
        nixpkgs.overlays = [fenix.overlays.complete];
        formatter = pkgs.writeShellApplication {
          name = "Formater";
          text = ''
            ${pkgs.alejandra}/bin/alejandra -q "$@"
            cargo fmt
            ${pkgs.fd}/bin/fd wgsl -X /home/sargo/.cargo/bin/wgslfmt {}
          '';
        };
        devShells = {
          default = mkShell {
            nativeBuildInputs = nativeBuildInputs ++ [rust_toolchain wgsl.packages.${system}.default];
            inherit buildInputs;
            LD_LIBRARY_PATH =
              nixpkgs.lib.makeLibraryPath (buildInputs ++ nativeBuildInputs);
          };
          fast = mkShell {
            nativeBuildInputs = nativeBuildInputs ++ [fast_rust_toolchain fast_run_command wgsl.packages.${system}.default];
            inherit buildInputs;
            LD_LIBRARY_PATH = nixpkgs.lib.makeLibraryPath (buildInputs ++ nativeBuildInputs);
          };
        };
      });
}
