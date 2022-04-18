with import <nixpkgs> {};
mkShell {
  buildInputs = [ cargo rustfmt rustc clippy cargo-edit gdb ];
}
