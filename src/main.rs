use bdk::bitcoin::consensus::serialize;
use bdk::bitcoin::Address;
use bdk::blockchain::Blockchain;
use bdk::blockchain::{noop_progress, ElectrumBlockchain};
use bdk::database::MemoryDatabase;
use bdk::electrum_client::Client;
use bdk::wallet::tx_builder;
use bdk::wallet::AddressIndex::New;
use bdk::Wallet;
use clap::crate_version;
use clap::{ArgGroup, Clap};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::rc::Rc;
use std::str::FromStr;

mod ur;
use ur::{is_ur_address, is_ur_descriptor, parse_ur_descriptor, psbt_as_ur};

mod errors;
use errors::SweepError;

fn parse_int(input: &str) -> Option<u32> {
    input
        .chars()
        .skip_while(|ch| !ch.is_digit(10))
        .take_while(|ch| ch.is_digit(10))
        .fold(None, |acc, ch| {
            ch.to_digit(10).map(|b| acc.unwrap_or(0) * 10 + b)
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

const ABOUT: &str = r#"Sweeptool creates a PSBT for the funds you want to sweep from a Bitcoin output descriptor.
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
#[clap(version=crate_version!(), about=ABOUT)]
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
}

fn main() -> Result<(), SweepError> {
    let opt = CliInput::parse();

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
            "UR descriptor cannot be currently passed via STDIN. Pass it as a CLI arg".to_string(),
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
            "UR descriptor cannot be currently passed via STDIN. Pass it as a CLI arg".to_string(),
        ));
    };

    let mut dest_addresses: Vec<String> = Vec::new();

    let mut client_url = "ssl://electrum.blockstream.info:60002";
    let netw = if opt.network == "mainnet" {
        bdk::bitcoin::Network::Bitcoin
    } else if opt.network == "testnet" {
        bdk::bitcoin::Network::Testnet
    } else {
        client_url = "127.0.0.1:51401";
        bdk::bitcoin::Network::Regtest
    };

    let client = Client::new(client_url)?;

    let wallet = Wallet::new(
        &descriptor,
        Some(&descriptor_chg),
        netw,
        MemoryDatabase::default(),
        ElectrumBlockchain::from(client),
    )?;

    // TEST
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
            ));
            //decode_ur_address(opt.address)?
        } else {
            Address::from_str(&addr)?
        };

        dest_addresses.push(addr.to_string());

        {
            let mut builder = wallet.build_tx();
            builder.drain_wallet();
            builder
                .set_single_recipient(addr.script_pubkey())
                .enable_rbf()
                .fee_rate(feerate);
            builder.finish()?
        }
    } else {
        // TODO remove this when STDIN support implemented
        let descriptor = {
            let desc = opt.dest_descriptor.unwrap();
            if is_ur_descriptor(desc.clone()) {
                // safe
                // this is UR format
                parse_ur_descriptor(desc.clone())?
            } else {
                // this is bitcoin core compatible format
                desc.to_string()
            }
        };

        // TODO remove this when STDIN support implemented
        let descriptor_chg = {
            let desc = opt.dest_descriptor_chg.unwrap();
            if is_ur_descriptor(desc.to_string()) {
                // this is UR format
                parse_ur_descriptor(desc.to_string())?
            } else {
                // this is bitcoin core compatible format
                desc.to_string()
            }
        };

        //println!("********** 123");

        // user is sweeping to an output descriptor
        let wallet_destination = Rc::new(Wallet::new_offline(
            &descriptor,
            None,
            netw,
            MemoryDatabase::default(),
        )?);

        let wallet_destination_chg = Rc::new(Wallet::new_offline(
            &descriptor_chg,
            None,
            netw,
            MemoryDatabase::default(),
        )?);

        fn get_child_indx<D: bdk::database::BatchDatabase, B>(
            w: Rc<Wallet<B, D>>,
            utxo: bdk::LocalUtxo,
            network: bdk::bitcoin::Network,
        ) -> Option<u32> {
            //println!("cc utxo {:?}", utxo.clone());
            for i in 0..20 {
                // TODO
                let addr = w.get_address(bdk::wallet::AddressIndex::Peek(i)).unwrap();

                let address = Address::from_script(
                    &utxo.txout.script_pubkey,
                    network, // TODO
                )
                .unwrap(); // TODO

                if addr == address {
                    //println!("cc addr {:?}", addr);
                    //println!("cc address {:?}", address);
                    return Some(i);
                }
            } //TODO
            None
        }

        let unspent = wallet.list_unspent().unwrap();
        //println!("unspent: {:?}", unspent);
        {
            let mut builder = wallet.build_tx();
            for u in &unspent {
                //println!("u.clone(): {:?}", u.clone());
                let indx = get_child_indx(Rc::clone(&wallet_source), u.clone(), netw);
                //println!("indx: {:?}", indx);
                let indx_chg = get_child_indx(Rc::clone(&wallet_source_chg), u.clone(), netw);
                //println!("indx_chg: {:?}", indx_chg);
                let address_dest = if let Some(d) = indx {
                    wallet_destination.get_address(bdk::wallet::AddressIndex::Peek(d))?
                } else if let Some(d) = indx_chg {
                    wallet_destination_chg.get_address(bdk::wallet::AddressIndex::Peek(d))?
                } else {
                    return Err(SweepError::new(
                        "bip32 index".to_string(),
                        "Address not found in output descriptor. Maybe increase the address gap"
                            .to_string(),
                    ));
                };

                dest_addresses.push(address_dest.to_string());

                //println!("fee: {:?}", fee);
                //println!("address_dest: {:?}", address_dest);
                builder
                    .manually_selected_only()
                    .add_utxo(u.outpoint)
                    .unwrap()
                    .ordering(tx_builder::TxOrdering::Untouched)
                    .add_recipient(address_dest.script_pubkey(), u.txout.value) // TODO script pubkey accoridng to new descirptor
                    .enable_rbf();
            }
            builder.fee_rate(feerate);
            let err = builder.finish();

            if let Err(e) = err {
                let mut err_str = e.to_string();
                let err_str = err_str.replace("InsufficientFunds { needed: ", "");
                let num = parse_int(&err_str).unwrap();

                println!("err_str {:?}", err_str);

                println!("num {:?}", num);

                println!("details {:?}", details);

                //println!("v {:?}", v);
                //let v: Value = serde_json::from_str(&err_str)?;
                //println!("v {:?}", v);
            }

            //let abs_fees = err.unwrap().needed - err.unwrap().available;

            // here we get the size of our Tx. Now we can determine the fees
            //let tx_weight = psbt.clone().extract_tx().get_weight();
            //let fees_abs_per_utxo = /*tx_weight * */ feerate; //TODO div

            let mut builder = wallet.build_tx();
            for u in unspent {
                //println!("u.clone(): {:?}", u.clone());
                let indx = get_child_indx(Rc::clone(&wallet_source), u.clone(), netw);
                //println!("indx: {:?}", indx);
                let indx_chg = get_child_indx(Rc::clone(&wallet_source_chg), u.clone(), netw);
                //println!("indx_chg: {:?}", indx_chg);
                let address_dest = if let Some(d) = indx {
                    wallet_destination.get_address(bdk::wallet::AddressIndex::Peek(d))?
                } else if let Some(d) = indx_chg {
                    wallet_destination_chg.get_address(bdk::wallet::AddressIndex::Peek(d))?
                } else {
                    return Err(SweepError::new(
                        "bip32 index".to_string(),
                        "Address not found in output descriptor. Maybe increase the address gap"
                            .to_string(),
                    ));
                };

                dest_addresses.push(address_dest.to_string());

                //println!("fee: {:?}", fee);
                //println!("address_dest: {:?}", address_dest);
                builder
                    .manually_selected_only()
                    .add_utxo(u.outpoint)
                    .unwrap()
                    .ordering(tx_builder::TxOrdering::Untouched)
                    .add_recipient(address_dest.script_pubkey(), u.txout.value) // TODO script pubkey accoridng to new descirptor
                    .enable_rbf();
            }
            //builder.fee_absolute(fee_combined as u64);
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

    Ok(())
}
