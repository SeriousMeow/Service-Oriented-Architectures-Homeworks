{
  description = "Discovery system";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";
    oas3-gen.url = "github:eklipse2k8/oas3-gen";
    oas3-gen.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs = { nixpkgs, oas3-gen, ... }:
    let
      system = "x86_64-linux";
      pkgs = nixpkgs.legacyPackages.${system};
    in
    {
      devShells.${system}.default = pkgs.mkShell {
        buildInputs = [
          pkgs.rustc
          pkgs.cargo
          pkgs.rustfmt
          oas3-gen.packages.${system}.oas3-gen

          pkgs.cmake
          pkgs.curl.dev
          pkgs.dockrtui
        ];
      };
    };
}
