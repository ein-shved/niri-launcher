{
  description = ''
    Simple utility to smartly launch different instruments withing niri
  '';

  outputs =
    { nixpkgs, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
        };
        niri-launcher = pkgs.callPackage ./. { };
      in
      {
        packages = {
          inherit niri-launcher;
          default = niri-launcher;
        };
        formatter = pkgs.nixfmt-rfc-style;
        devShells.default = pkgs.mkShell {
          inputsFrom = [ niri-launcher ];
          packages = with pkgs; [
            rust-analyzer
            rustfmt
          ];
        };
      }
    );
}
