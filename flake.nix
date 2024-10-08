{
  description = "Space game";

  inputs = {
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
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
    wgsl,
  }:
    flake-utils.lib.eachDefaultSystem (system: let
      pkgs = nixpkgs.legacyPackages.${system};
      linker_pkgs = with pkgs; [mold clang];

      rust_build_components = ["rustc" "cargo" "rust-src" "rust-std"];
      rust_dev_components = ["clippy" "rustfmt" "rust-analyzer"] ++ rust_build_components;

      rust_toolchain = fenix.packages.${system}.stable.withComponents rust_dev_components;

      fast_rust_toolchain =
        fenix.packages.${system}.latest.withComponents
        (rust_dev_components ++ ["rustc-codegen-cranelift-preview"]);

      fast_run_command = pkgs.writeShellApplication {
        name = "frun";
        text = ''
          RUSTFLAGS="-Z threads=10 -C target-cpu=native -Zcodegen-backend=cranelift" cargo run \$@
        '';
      };

      nativeBuildInputs = with pkgs; [pkg-config cmake pkg-config freetype expat fontconfig wasm-bindgen-cli] ++ linker_pkgs;
      buildInputs = with pkgs; [lldb_15 udev alsa-lib vulkan-loader libxkbcommon wayland];
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
