{ lib, source ? lib.cleanSource ./., rustPlatform }:

rustPlatform.buildRustPackage rec {
  pname = "prune-rs";
  version = "unstable-2022-05-04";

  src = source;
  cargoLock.lockFile = ./Cargo.lock;
}
