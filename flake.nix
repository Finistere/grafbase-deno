{
  description = "Grafbase CLI development environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = {
    self,
    nixpkgs,
    flake-utils,
    ...
  }: let
    inherit (nixpkgs.lib) optional;
    systems = flake-utils.lib.system;
  in
    flake-utils.lib.eachDefaultSystem (system: let
      pkgs = import nixpkgs {
        inherit system;
      };
    in {
      devShells.default = pkgs.mkShell {
        nativeBuildInputs = with pkgs;
          [
            cargo-make
            rustup
            openssl
            pkg-config
            deno
          ]
          ++ optional (system == systems.aarch64-darwin) [
            darwin.apple_sdk.frameworks.CoreFoundation
            darwin.apple_sdk.frameworks.CoreServices
            darwin.apple_sdk.frameworks.Security
          ];

        shellHook = ''
          project_root="$(git rev-parse --show-toplevel 2>/dev/null || jj workspace root 2>/dev/null)"
          export DENO_INSTALL_ROOT="$project_root/.deno";
          export CARGO_INSTALL_ROOT="$project_root/.cargo";
          export PATH="$DENO_INSTALL_ROOT/bin:$CARGO_INSTALL_ROOT/bin:$project_root/node_modules/.bin:$PATH";
          cargo install --offline wasm-bindgen-cli@0.2.86 2>/dev/null || cargo install wasm-bindgen-cli@0.2.86
        '';
      };
    });
}
