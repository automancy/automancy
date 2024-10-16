{
  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    crane.url = "github:ipetkov/crane";
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs = { self, flake-utils, crane, nixpkgs }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = (import nixpkgs) { inherit system; };
        inherit (pkgs) lib;

        craneLib = crane.mkLib pkgs;

        buildInputs = with pkgs; [
          alsa-lib
          libxkbcommon
          vulkan-loader
          wayland
          xorg.libX11
          libGL
          # a few of these are probably extra
          xorg.libXcursor
          xorg.libXi
          xorg.libXrandr
        ];

        binInputs = with pkgs; [
          zenity
        ];

        # Common arguments can be set here to avoid repeating them later
        # Note: changes here will rebuild all dependency crates
        commonArgs = {
          src = ./.;
          strictDeps = true;

          nativeBuildInputs = with pkgs; [ pkg-config blender makeBinaryWrapper breakpointHook ];
          inherit buildInputs;
        };

        my-crate = craneLib.buildPackage (commonArgs // {
          pname = "automancy";
          cargoExtraArgs = "-p automancy";
          cargoArtifacts = craneLib.buildDepsOnly commonArgs;

          # Additional environment variables or build phases/hooks can be set
          # here *without* rebuilding all dependency crates
          # MY_CUSTOM_VAR = "some value";

          postInstall = ''
            wrapProgram "$out/bin/automancy" \
              --prefix LD_LIBRARY_PATH : ${lib.makeLibraryPath buildInputs } \
              --prefix PATH : "${lib.makeBinPath binInputs}"

            # TODO? --suffix XDG_DATA_DIRS : "$${cosmic-icons}/share" \
          '';
        });

      in rec {
        # For `nix build` & `nix run`:
        defaultPackage = my-crate;

        # For `nix develop`:
        devShell = pkgs.mkShell {
          nativeBuildInputs = with pkgs; [
            rustc
            cargo
            rust-analyzer
            pkg-config
            blender
          ] ++ binInputs;
          inherit buildInputs;
          LD_LIBRARY_PATH = lib.makeLibraryPath buildInputs;
        };
      });
}
