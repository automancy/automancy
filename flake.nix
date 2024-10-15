{
  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    naersk.url = "github:nix-community/naersk";
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs = { self, flake-utils, naersk, nixpkgs }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = (import nixpkgs) { inherit system; };
        inherit (pkgs) lib;

        naersk' = pkgs.callPackage naersk { };

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
          gnome.zenity
        ];

      in rec {
        # For `nix build` & `nix run`:
        defaultPackage = naersk'.buildPackage {
          src = ./.;
          nativeBuildInputs = with pkgs; [ pkg-config blender makeBinaryWrapper ];
          inherit buildInputs;

          cargoBuildOptions = prev: prev ++ [ "--package automancy" ];

          postInstall = ''
            wrapProgram "$out/bin/automancy" \
              --prefix LD_LIBRARY_PATH : ${lib.makeLibraryPath buildInputs } \
              --prefix PATH : "${lib.makeBinPath binInputs}"

            # TODO? --suffix XDG_DATA_DIRS : "$${cosmic-icons}/share" \
          '';
        };

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
