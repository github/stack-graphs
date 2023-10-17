{
  inputs.nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
  inputs.nci.url = "github:yusdacra/nix-cargo-integration";
  inputs.nci.inputs.nixpkgs.follows = "nixpkgs";
  inputs.parts.url = "github:hercules-ci/flake-parts";
  inputs.parts.inputs.nixpkgs-lib.follows = "nixpkgs";

  outputs = inputs @ {
    parts,
    nci,
    ...
  }:
    parts.lib.mkFlake {inherit inputs;} {
      systems = ["x86_64-linux"];
      imports = [nci.flakeModule];
      perSystem = {
        config,
        pkgs,
        lib,
        ...
      }: let
        # shorthand for accessing outputs
        # you can access crate outputs under `config.nci.outputs.<crate name>` (see documentation)
        outputs = config.nci.outputs;
      in {
        # declare projects
        # relPath is the relative path of a project to the flake root
        # TODO: change this to your crate's path
        nci.projects."stack-graphs" = {
          relPath = "";
          # export all crates (packages and devshell) in flake outputs
          # alternatively you can access the outputs and export them yourself
          export = true;
        };
        # configure crates
        nci.crates = {
          "stack-graphs" = {
            overrides.add-inputs.overrideAttrs = old: {
              buildInputs = (old.buildInputs or []) ++ [pkgs.sqlite];
            };
          };
          "tree-sitter-stack-graphs" = {
          };
          "lsp-positions" = {};
        };
        # export the project devshell as the default devshell
        devShells.default = outputs."stack-graphs".devShell;

        packages = let
          packages = [
            "tree-sitter-stack-graphs"
            "stack-graphs"
            "tree-sitter-stack-graphs-java"
            "tree-sitter-stack-graphs-typescript"
          ];
        in
          lib.listToAttrs (map (s: {
              name = s;
              value = outputs."${s}".packages.release;
            })
            packages)
          // {
            # export the release package of the crate as default package
            default = outputs."tree-sitter-stack-graphs".packages.release;
          };
      };
    };
}
