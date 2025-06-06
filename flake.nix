{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = {
    self,
    nixpkgs,
    rust-overlay,
  }: let
    system = "x86_64-linux";
    pkgs = import nixpkgs {
      inherit system;
      overlays = [rust-overlay.overlays.default];
    };
    toolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
  in {
    devShells.${system}.default = pkgs.mkShell {
      packages = [
        toolchain
        pkgs.rust-analyzer-unwrapped
        pkgs.qemu
        pkgs.nasm
        pkgs.grub2
      ];

      RUST_SRC_PATH = "${toolchain}/lib/rustlib/src/rust/library";

      shellHook = ''
        export PATH="/home/$(whoami)/.cargo/bin:$PATH"
      '';
    };
  };
}
