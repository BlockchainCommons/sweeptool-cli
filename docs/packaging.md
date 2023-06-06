# How to build a Debian package.

You should be in a checked-out `debian/latest` branch.

The simplest tool to use to build Rust-based Debian packages is `cargo-deb`, installed with:
```sh
rustup update   
cargo install cargo-deb
```

Download and build [openssl-1.1](https://www.openssl.org/source/openssl-1.1.1u.tar.gz)
`apt get libssl-dev` installs 3.0 and there is no way to use apt to get libssl1.1 header files.

Then build the package with:

```sh
OPENSSL_DIR=$HOME/tmp/openssl-1.1.1t OPENSSL_LIB_DIR=tmp/openssl-1.1.1t cargo deb
```

This will produce a `.deb` package in the `./target/debian` directory.
