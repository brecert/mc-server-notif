{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    crane.url = "github:ipetkov/crane";
    crane.inputs.nixpkgs.follows = "nixpkgs";
    flake-utils.url = "github:numtide/flake-utils";

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        flake-utils.follows = "flake-utils";
      };
    };
  };

  outputs = { self, nixpkgs, crane, flake-utils, rust-overlay, ... }:
    flake-utils.lib.eachDefaultSystem (system: 
    let
      pkgs = import nixpkgs {
        inherit system;
        overlays = [ (import rust-overlay) ];
      };

      rustNightlyTarget = pkgs.rust-bin.selectLatestNightlyWith (toolchain: toolchain.default);

      craneLib = (crane.mkLib pkgs).overrideToolchain rustNightlyTarget;

      crate = craneLib.buildPackage {
        src = ./.;

        # Add extra inputs here or any other derivation settings
        # doCheck = true;
        # buildInputs = [];
        nativeBuildInputs = with pkgs; [pkg-config glib gdk-pixbuf pango atk gtk3];
      };
    in
    {
      defaultPackage = crate;
      defaultApp = crate;
    });
}
