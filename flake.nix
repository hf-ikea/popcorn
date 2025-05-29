{
  description = "Checks chapters against latest";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = {
    self,
    nixpkgs,
    rust-overlay,
    flake-utils,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (
      system: let
        overlays = [(import rust-overlay)];
        pkgs = import nixpkgs {inherit system overlays;};

        buildInputs = with pkgs; [rustup qemu nasm grub2];
        nativeBuildInputs = with pkgs; [libgpg-error gpgme pkg-config];
        cargoToml = builtins.fromTOML (builtins.readFile (self + /Cargo.toml));
        rustToolchain = pkgs.rust-bin.fromRustupToolchainFile (self + /rust-toolchain.toml);
        inherit (cargoToml.package) name version;
      in {
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = name;
          version = version;
          src = pkgs.lib.cleanSource self;
          cargoLock.lockFile = self + /Cargo.lock;
          rustToolchain = rustToolchain;
          buildInputs = buildInputs;
          nativeBuildInputs = nativeBuildInputs;
        };

        devShells.default = with pkgs;
          mkShell {
            buildInputs = [rustToolchain] ++ buildInputs;
            nativeBuildInputs = nativeBuildInputs;

            shellHook = ''
              export PATH="/home/$(whoami)/.cargo/bin:$PATH"
            '';
          };
      }
    );
}
