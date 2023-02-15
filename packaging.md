# How to build a Debian package.

You should be in a checked-out `debian/latest` branch.

The simplest tool to use to build Rust-based Debian packages is `cargo-deb`, installed with:
```sh
cargo install cargo-deb
```
Then build the package with:

```sh
cargo deb
```
