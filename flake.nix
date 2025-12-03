{
  description = "git-trending-motd - Trending repositories of the day";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
      in
      {
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "git-trending-motd";
          version = "0.0.2";
          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;

          meta = with pkgs.lib; {
            description = "Trending repositories of the day - minimal MOTD CLI";
            homepage = "https://github.com/schausberger/git-trending-motd";
            license = licenses.mit;
            maintainers = [ "schausberger" ];
          };
        };

        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            # Rust toolchain
            rustc
            cargo
            rustfmt
            clippy
            rust-analyzer

            # Development tools
            cargo-nextest
            cargo-watch

            # Pre-commit hooks
            prek

            # Build dependencies
            pkg-config
            openssl
          ];

          shellHook = ''
            echo "✓ git-trending-motd development environment"
            echo "  cargo build    - Build the project"
            echo "  cargo test     - Run tests"
            echo "  cargo nextest run - Run tests with nextest"
            echo "  cargo run      - Run git-trending"
            echo "  prek run --all-files - Run pre-commit hooks"
          '';
        };
      }
    );
}
