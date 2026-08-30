{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";

    flake-parts.url = "github:hercules-ci/flake-parts";
    flake-parts.inputs.nixpkgs-lib.follows = "nixpkgs";

    rust-overlay.url = "github:oxalica/rust-overlay";
    rust-overlay.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs = inputs @ {flake-parts, ...}:
    flake-parts.lib.mkFlake {inherit inputs;} {
      systems = [
        "x86_64-linux"
      ];
      perSystem = {
        system,
        pkgs,
        ...
      }: {
        _module.args.pkgs = import inputs.nixpkgs {
          inherit system;
          overlays = [
            inputs.rust-overlay.overlays.default
          ];
          config = {};
        };

        devShells.default =
          pkgs.mkShell
          .override {
            stdenv = pkgs.stdenvAdapters.useMoldLinker pkgs.clangStdenv;
          }
          {
            packages = with pkgs.rust-bin;
              [
                (stable.latest.default.override {
                  extensions = ["rust-src"];
                })
                # stable.latest.default
                stable.latest.clippy

                nightly.latest.rustfmt
                nightly.latest.rust-analyzer
              ]
              ++ (with pkgs; [
                mold
              ]);
          };
      };
    };
}
