{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let pkgs = nixpkgs.legacyPackages.${system}; in
      {
        devShell = pkgs.mkShell rec {
          nativeBuildInputs = with pkgs; [
            pkgconfig
            llvmPackages.bintools # To use lld linker
          ];
          buildInputs = with pkgs; [
            udev
            alsaLib
            vulkan-loader
            xlibsWrapper
            xorg.libXcursor
            xorg.libXrandr
            xorg.libXi # To use x11 feature
            libxkbcommon
            wayland # To use wayland feature
          ];
          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath buildInputs;
        };
      });
}
