use bdk::bitcoin::consensus::serialize;
use bdk::bitcoin::Address;
use bdk::blockchain::esplora::EsploraBlockchainConfig;
use bdk::blockchain::noop_progress;
use bdk::blockchain::Blockchain;
use bdk::blockchain::{
    AnyBlockchain, AnyBlockchainConfig, ConfigurableBlockchain, ElectrumBlockchainConfig,
};
use bdk::database::MemoryDatabase;
use bdk::wallet::tx_builder;
use bdk::{SignOptions, Wallet};
use clap::crate_version;
use clap::{ArgGroup, Clap};
use serde::{Deserialize, Serialize};
use std::rc::Rc;
use std::str::FromStr;

mod ur;
use ur::{is_ur_address, is_ur_descriptor, parse_ur_descriptor, psbt_as_ur};

mod errors;
use errors::SweepError;

// parse the first integer in a string
fn parse_int(input: &str) -> Option<u64> {
    input
        .chars()
        .skip_while(|ch| !ch.is_digit(10))
        .take_while(|ch| ch.is_digit(10))
        .fold(None, |acc, ch| {
            ch.to_digit(10).map(|b| acc.unwrap_or(0) * 10 + b as u64)
        })
}

#[derive(Serialize, Deserialize, Debug)]
struct Psbt {
    base64: String,
    ur: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct CliOutput {
    amount: u64,
    fees: u64,
    address: Vec<String>,
    timestamp: u64,
    txid: String,
    psbt: Psbt,
}

// TODO
const _ABOUT: &str = r#"Sweeptool creates a PSBT for the funds you want to sweep from a Bitcoin output descriptor.
Result:
{                       (json object)
  "amount" : n,         (numeric) amount swept
  "fees" : n,           (numeric) miner fees [sats]
  "address" : ["str"]   (array of strings) destination address(es)
  "timestamp": n,       (numeric) unix timestamp of the PSBT created
  "txid" : "str",       (string) Transaction ID
  "psbt" : {            (json object)
     "base64" : "str",  (string) psbt in base64 format
     "ur" : "str"       (string) psbt in UR format
   }
}
"#;

#[derive(Clap, Debug)]
#[clap(verbatim_doc_comment)]
#[clap(group = ArgGroup::new("destination").required(true))]
struct CliInput {
    /// Descriptor in UR format or in Bitcoin Core compatible format
    #[clap(short = 'd')]
    descriptor: String,
    /// Change descriptor in UR format or in Bitcoin core compatible format
    #[clap(short = 'c')]
    descriptor_chg: String,
    /// Address gap limit to search within for available funds
    #[clap(short = 'g', default_value = "20")]
    address_gap_limit: u32,
    /// Bitcoin address in UR format or in Bitcoin Core compatible format.
    #[clap(short, group = "destination")]
    address: Option<String>,
    /// Destination descriptor in UR format or in Bitcoin Core compatible format
    #[clap(short = 'e', group = "destination", requires = "dest-descriptor-chg")]
    dest_descriptor: Option<String>,
    /// Destination change descriptor in UR format or in Bitcoin core compatible format
    #[clap(short = 's')]
    dest_descriptor_chg: Option<String>,
    /// Target (number of blocks) used to estimate the fee rate for a PSBT
    #[clap(short, default_value = "6")]
    target: usize,
    /// Bitcoin network
    #[clap(short, default_value = "testnet", possible_values=&["mainnet", "testnet", "regtest"])]
    network: String,
    /// By default electrum server is used ssl://electrum.blockstream.info:60002 to query blockchain.
    /// But you can override it with an esplora server of your choice
    /// Examples: https://blockstream.info/testnet/api for testnet and https://blockstream.info/api for mainnet
    #[clap(short = 'p', long, conflicts_with = "server")]
    esplora: Option<String>,
    /// Electrum server to query the blockchain. Default="ssl://electrum.blockstream.info:60002"
    /// In regtest mode 127.0.0.1:51401 is hardcoded.
    #[clap(long, default_value = "ssl://electrum.blockstream.info:60002")]
    server: String,
    /// You can pass a proxy e.g. localhost:9050 and then pass an onion address of an Electrum server
    /// to the server arg, e.g.
    /// explorerzydxu5ecjrkwceayqybizmpjjznk5izmitf2modhcusuqlid.onion:143 for testnet
    #[clap(long, conflicts_with = "esplora")]
    proxy: Option<String>,
}

#[derive(Clap, Debug)]
struct SignPSBT {
    /// Private descriptor in Bitcoin Core compatible format
    #[clap(short = 'd')]
    descriptor: String,
    /// Private change descriptor in Bitcoin core compatible format
    #[clap(short = 'c')]
    descriptor_chg: String,
    /// PSBT in Bitcoin Core compatible format
    #[clap(required = true)]
    psbt: String,
    /// Bitcoin network
    #[clap(short, default_value = "testnet", possible_values=&["mainnet", "testnet", "regtest"])]
    network: String,
}

#[derive(Clap, Debug)]
#[clap(version=crate_version!())]
enum Opt {
    /// Sweep funds
    Sweep(CliInput),
    /// Sign a PSBT
    Sign(SignPSBT),
}

fn main() -> Result<(), SweepError> {
    let matches = Opt::parse();

    match matches {
        Opt::Sign(cmd) => {
            let netw = if cmd.network == "mainnet" {
                bdk::bitcoin::Network::Bitcoin
            } else {
                bdk::bitcoin::Network::Testnet
            };

            use bdk::bitcoin::consensus::deserialize;
            let wallet = Wallet::new_offline(
                &cmd.descriptor,
                Some(&cmd.descriptor_chg),
                netw,
                MemoryDatabase::default(),
            )?;

            use bdk::bitcoin::util::psbt::PartiallySignedTransaction;
            let mut psbt: PartiallySignedTransaction =
                deserialize(&base64::decode(&cmd.psbt).unwrap()).unwrap();

            let _finalized = wallet.sign(&mut psbt, SignOptions::default())?;

            let out = Psbt {
                base64: base64::encode(&serialize(&psbt)),
                ur: psbt_as_ur(serialize(&psbt))?,
            };

            println!("{}", serde_json::to_string(&out)?);
        }
        Opt::Sweep(opt) => {
            // TODO remove this when STDIN support implemented
            let descriptor = Some(opt.descriptor);
            let descriptor = if let Some(ref desc) = descriptor {
                if is_ur_descriptor(desc.to_string()) {
                    // this is UR format
                    parse_ur_descriptor(desc.to_string())?
                } else {
                    // this is bitcoin core compatible format
                    desc.to_string()
                }
            } else {
                return Err(SweepError::new(
                    "cli arg".to_string(),
                    "UR descriptor cannot be currently passed via STDIN. Pass it as a CLI arg"
                        .to_string(),
                ));
            };

            // TODO remove this when STDIN support implemented
            let descriptor_chg = Some(opt.descriptor_chg);
            let descriptor_chg = if let Some(ref desc) = descriptor_chg {
                if is_ur_descriptor(desc.to_string()) {
                    // this is UR format
                    parse_ur_descriptor(desc.to_string())?
                } else {
                    // this is bitcoin core compatible format
                    desc.to_string()
                }
            } else {
                return Err(SweepError::new(
                    "cli arg".to_string(),
                    "UR descriptor cannot be currently passed via STDIN. Pass it as a CLI arg"
                        .to_string(),
                ));
            };

            let mut dest_addresses: Vec<String> = Vec::new();

            let mut client_url = opt.server;
            let netw = if opt.network == "mainnet" {
                bdk::bitcoin::Network::Bitcoin
            } else if opt.network == "testnet" {
                bdk::bitcoin::Network::Testnet
            } else {
                client_url = "127.0.0.1:51401".to_string();
                bdk::bitcoin::Network::Regtest
            };

            let config_electrum = AnyBlockchainConfig::Electrum(ElectrumBlockchainConfig {
                url: client_url,
                socks5: opt.proxy,
                retry: 2,
                timeout: None,
            });

            let config_esplora = opt.esplora.map(|e| {
                AnyBlockchainConfig::Esplora(EsploraBlockchainConfig {
                    base_url: e,
                    concurrency: Some(4),
                })
            });

            let config = config_esplora.unwrap_or(config_electrum);

            let wallet = Wallet::new(
                &descriptor,
                Some(&descriptor_chg),
                netw,
                MemoryDatabase::default(),
                AnyBlockchain::from_config(&config)?,
            )?;

            // user is sweeping to an output descriptor
            let wallet_source = Rc::new(Wallet::new_offline(
                &descriptor,
                None,
                netw,
                MemoryDatabase::default(),
            )?);

            let wallet_source_chg = Rc::new(Wallet::new_offline(
                &descriptor_chg,
                None,
                netw,
                MemoryDatabase::default(),
            )?);

            let feerate = wallet.client().estimate_fee(opt.target)?;

            wallet.sync(noop_progress(), Some(opt.address_gap_limit))?;

            // Is user sweeping to an address or to an output descriptor?
            let (psbt, details) = if let Some(ref addr) = opt.address {
                let addr = if is_ur_address(addr.to_string()) {
                    return Err(SweepError::new(
                        "cli arg".to_string(),
                        "UR address not implemented".to_string(),
                    )); // TODO
                        //decode_ur_address(opt.address)?
                } else {
                    Address::from_str(&addr)?
                };

                dest_addresses.push(addr.to_string());

                {
                    // build a PSBT sweeping to an address
                    let mut builder = wallet.build_tx();
                    builder.drain_wallet();
                    builder
                        .set_single_recipient(addr.script_pubkey())
                        .enable_rbf()
                        .fee_rate(feerate);
                    builder.finish()?
                }
            } else {
                // build a PSBT sweeping to an output descriptor
                // We are gonna prepare here individual wallets (descriptor, descriptor_chg, descriptor_destination,
                // descriptor_destination_chg) so we can easily
                // search for address indices when mapping UTXOs from a source descriptor to a
                // destination descriptor

                // TODO remove this when STDIN support implemented
                let descriptor = {
                    let desc = opt.dest_descriptor.unwrap();
                    if is_ur_descriptor(desc.clone()) {
                        // safe
                        // this is UR format
                        parse_ur_descriptor(desc)?
                    } else {
                        // this is bitcoin core compatible format
                        desc
                    }
                };

                // TODO remove this when STDIN support implemented
                let descriptor_chg = {
                    let desc = opt.dest_descriptor_chg.unwrap();
                    if is_ur_descriptor(desc.to_string()) {
                        // this is UR format
                        parse_ur_descriptor(desc)?
                    } else {
                        // this is bitcoin core compatible format
                        desc
                    }
                };

                // user is sweeping to an output descriptor
                let descriptor_destination = Rc::new(Wallet::new_offline(
                    &descriptor,
                    None,
                    netw,
                    MemoryDatabase::default(),
                )?);

                let descriptor_destination_chg = Rc::new(Wallet::new_offline(
                    &descriptor_chg,
                    None,
                    netw,
                    MemoryDatabase::default(),
                )?);

                fn get_child_indx<D: bdk::database::BatchDatabase, B>(
                    w: Rc<Wallet<B, D>>,
                    utxo: bdk::LocalUtxo,
                    network: bdk::bitcoin::Network,
                    address_gap_limit: u32,
                ) -> Option<u32> {
                    for i in 0..address_gap_limit {
                        let addr = w.get_address(bdk::wallet::AddressIndex::Peek(i)).unwrap();
                        let address =
                            Address::from_script(&utxo.txout.script_pubkey, network).unwrap(); // TODO
                        if addr.address == address {
                            return Some(i);
                        }
                    }
                    None
                }

                let unspent = wallet.list_unspent().unwrap();

                {
                    // here we construct a psbt with zero fees so we can determine Tx size
                    // Based on Tx size we can construct a rael psbt with real fees in the next stage
                    let mut builder = wallet.build_tx();
                    for u in &unspent {
                        let indx = get_child_indx(
                            Rc::clone(&wallet_source),
                            u.clone(),
                            netw,
                            opt.address_gap_limit,
                        );
                        let indx_chg = get_child_indx(
                            Rc::clone(&wallet_source_chg),
                            u.clone(),
                            netw,
                            opt.address_gap_limit,
                        );
                        let address_dest = if let Some(d) = indx {
                            descriptor_destination
                                .get_address(bdk::wallet::AddressIndex::Peek(d))?
                        } else if let Some(d) = indx_chg {
                            descriptor_destination_chg
                                .get_address(bdk::wallet::AddressIndex::Peek(d))?
                        } else {
                            return Err(SweepError::new(
                        "bip32 index".to_string(),
                        "Address not found in output descriptor. Maybe increase the address gap"
                            .to_string(),
                    ));
                        };

                        dest_addresses.push(address_dest.to_string());

                        builder
                            .manually_selected_only()
                            .add_utxo(u.outpoint)
                            .unwrap()
                            .ordering(tx_builder::TxOrdering::Untouched)
                            .add_recipient(address_dest.script_pubkey(), u.txout.value)
                            .enable_rbf();
                    }
                    builder.fee_rate(feerate);
                    let err = builder.finish();

                    let fee_per_utxo = if let Err(e) = err {
                        let err_str = e.to_string();

                        let split = err_str.split(',');
                        let vec = split.collect::<Vec<&str>>();

                        let needed = parse_int(&vec[0]).unwrap();
                        let available = parse_int(&vec[1]).unwrap();

                        (needed - available) as u64 / unspent.len() as u64
                    } else {
                        panic!("fees error");
                    };

                    // Now  we can construct a PSBT with real fees:
                    let mut builder = wallet.build_tx();
                    let mut fee_combined = 0;
                    for u in &unspent {
                        let indx = get_child_indx(
                            Rc::clone(&wallet_source),
                            u.clone(),
                            netw,
                            opt.address_gap_limit,
                        );
                        let indx_chg = get_child_indx(
                            Rc::clone(&wallet_source_chg),
                            u.clone(),
                            netw,
                            opt.address_gap_limit,
                        );
                        let address_dest = if let Some(d) = indx {
                            descriptor_destination
                                .get_address(bdk::wallet::AddressIndex::Peek(d))?
                        } else if let Some(d) = indx_chg {
                            descriptor_destination_chg
                                .get_address(bdk::wallet::AddressIndex::Peek(d))?
                        } else {
                            return Err(SweepError::new(
                        "bip32 index".to_string(),
                        "Address not found in output descriptor. Maybe increase the address gap limit"
                            .to_string(),
                    ));
                        };

                        dest_addresses.push(address_dest.to_string());

                        let recipient_amount = if u.txout.value > fee_per_utxo {
                            fee_combined += fee_per_utxo;
                            u.txout.value - fee_per_utxo
                        } else {
                            fee_combined += u.txout.value;
                            0
                        };

                        builder
                            .manually_selected_only()
                            .add_utxo(u.outpoint)
                            .unwrap()
                            .ordering(tx_builder::TxOrdering::Untouched)
                            .add_recipient(address_dest.script_pubkey(), recipient_amount)
                            .enable_rbf();
                    }
                    builder.fee_absolute(fee_combined);
                    builder.finish()?
                }
            };

            /*
                println!(
                    "DEBUG psbt: {}",
                    serde_json::to_string_pretty(&psbt).unwrap()
                );
            */

            let out = CliOutput {
                amount: details.sent,
                fees: details.fees,
                address: dest_addresses,
                timestamp: details.timestamp,
                txid: details.txid.to_string(),
                psbt: Psbt {
                    base64: base64::encode(&serialize(&psbt)),
                    ur: psbt_as_ur(serialize(&psbt))?,
                },
            };

            println!("{}", serde_json::to_string(&out)?);
        }
    }

    Ok(())
}
