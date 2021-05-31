## Manual



### Help

```bash
$ sweeptool -h
sweeptool-cli 0.1.0

USAGE:
    sweeptool [OPTIONS] -a <address>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -a <address>                  Bitcoin address in UR format or in Bitcoin Core compatible format
    -g <address-gap-limit>        Address gap limit to search within for available funds
    -d <descriptor>               Descriptor in UR format or in Bitcoin Core compatible format
    -c <descriptor-chg>           Change descriptor in UR format or in Bitcoin core compatible
                                  format
    -n <network>                  Bitcoin network [default: testnet] [possible values: mainnet,
                                  testnet]
    -t <target>                   Target (number of blocks) used to estimate the fee rate for a PSBT
                                  [default: 6]
```
