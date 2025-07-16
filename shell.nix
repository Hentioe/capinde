with import <nixpkgs> { };

stdenv.mkDerivation {
  name = "build";
  buildInputs = [
    pkg-config
    imagemagick
    open-sans
  ];

  LIBCLANG_PATH = pkgs.lib.makeLibraryPath [ pkgs.llvmPackages_latest.libclang.lib ];

  shellHook = ''
    export XDG_DATA_DIRS=$XDG_DATA_DIRS:${pkgs.open-sans}/share
  '';
}
