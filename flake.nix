{
  description = "Union Square development environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      rust-overlay,
      flake-utils,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        rustToolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;

      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            rustToolchain
            lefthook
            just
            nodejs_22
            bun
            glow
            jq
            ast-grep
            sqlx-cli
            postgresql
          ];

          RUST_SRC_PATH = "${rustToolchain}/lib/rustlib/src/rust/library";

          shellHook = ''
            repo_root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
            dependenciesDir="$repo_root/.dependencies"

            export CARGO_HOME="$dependenciesDir/cargo"
            export CARGO_INSTALL_ROOT="$dependenciesDir/cargo"
            export npm_config_prefix="$dependenciesDir/npm"
            export BUN_INSTALL="$dependenciesDir/bun"

            mkdir -p "$CARGO_HOME/bin" "$npm_config_prefix/bin" "$BUN_INSTALL/bin"
            export PATH="$CARGO_HOME/bin:$npm_config_prefix/bin:$BUN_INSTALL/bin:$PATH"
          '';
        };
      }
    );
}
