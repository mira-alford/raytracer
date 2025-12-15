{
  inputs = {
    crane = {
      url = "github:ipetkov/crane";
      inputs = {
        flake-utils.follows = "flake-utils";
        nixpkgs.follows = "nixpkgs";
      };
    };
    rust-overlay.url = "github:oxalica/rust-overlay";

    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils.url = "github:numtide/flake-utils";
    nixpkgs.url = "nixpkgs/nixos-unstable";
  };

  outputs =
    {
      self,
      crane,
      fenix,
      flake-utils,
      nixpkgs,
      rust-overlay,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          config.allowUnfree = true;
          overlays = [ (import rust-overlay) ];
        };

        rust-pkgs = fenix.packages.${system}.stable;

        rustToolchain = rust-pkgs.withComponents [
          "cargo"
          "clippy"
          "rust-src"
          "rustc"
          "rustfmt"
        ];

        craneLib = (crane.mkLib pkgs).overrideToolchain (rustToolchain);

        runtimeDeps = (
          with pkgs;
          [
            pkg-config
            libxkbcommon
            alsa-lib
            udev
            wayland
            vulkan-loader
          ]
          ++ (with xorg; [
            libXcursor
            libXrandr
            libXi
            libX11
          ])
        );

        unfilteredRoot = ./.;
        lib = pkgs.lib;
        src = lib.fileset.toSource {
          root = unfilteredRoot;
          fileset = lib.fileset.unions [
            (craneLib.fileset.commonCargoSources unfilteredRoot)
            (lib.fileset.fileFilter (file: file.hasExt "slang") unfilteredRoot)
            (lib.fileset.maybeMissing ./shaders)
          ];
        };

        runner = craneLib.buildPackage {
          inherit src;
          # pname = "runner";
          cargoExtraArgs = "--workspace";
        };
      in
      {
        packages.default = runner;

        src = src;

        devShells.default = craneLib.devShell {
          # RUST_SRC_PATH = "${pkgs.rustPlatform.rustLibSrc}";
          LD_LIBRARY_PATH = "${pkgs.lib.makeLibraryPath runtimeDeps}";

          hardeningDisable = [ "fortify" ];

          # inputsFrom = [ runner ];

          packages =
            (with pkgs; [
              rust-analyzer
              wgsl-analyzer
              just
              shader-slang
              spirv-tools
            ])
            ++ runtimeDeps;
        };
      }
    );
}
