# How to build a Debian package.

If you can read this file, you should be in a checked-out `debian/latest` branch.

0. Apply the patches listed in `debian/patches/series`. They should apply cleanly before the repository can be compiled.
```sh
cat debian/series | xargs -I{} patch --strip=1 -i "debian/patches/{}"
```
Or

```sh
while read -r f; do
    patch -i "debian/patches/$f"
done < debian/series
```

In order to compile Sweeptool on Debian, [OpenSSL 1.1.1](https://www.openssl.org/source/openssl-1.1.1t.tar.gz) is required. After downloading and compiling it, compile sweeptool with a command in the form: 
7
```sh
OPENSSL_DIR=$HOME/wip/openssl-1.1.1t OPENSSL_LIB_DIR=$HOME/wip/openssl-1.1.1t cargo build
```
