{
  rustToolchain,
  cargoArgs,
  unitTestArgs,
  pkgs,
  lib,
  stdenv,
  darwin,
  ...
}:

let
  cargo-ext = pkgs.callPackage ./cargo-ext.nix { inherit cargoArgs unitTestArgs; };
in
pkgs.mkShell {
  name = "dev-shell";

  nativeBuildInputs = with pkgs; [
    cargo-ext.cargo-build-all
    cargo-ext.cargo-clippy-all
    cargo-ext.cargo-doc-all
    cargo-ext.cargo-nextest-all
    cargo-ext.cargo-test-all
    cargo-nextest
    rustToolchain

    tokei

    protobuf

    jq

    hclfmt
    nixfmt-rfc-style
    nodePackages.prettier
    sleek
    shfmt
    taplo
    treefmt
    # clang-tools contains clang-format
    clang-tools

    shellcheck

    pkg-config
    libgit2
    openssl

    sqlx-cli
  ];

  shellHook = ''
    export NIX_PATH="nixpkgs=${pkgs.path}"
  '';

  PROTOC = "${pkgs.protobuf}/bin/protoc";
  PROTOC_INCLUDE = "${pkgs.protobuf}/include";
}
