use bdk::bitcoin::consensus::serialize;
use bdk::bitcoin::Address;
use bdk::blockchain::Blockchain;
use bdk::blockchain::{noop_progress, ElectrumBlockchain};
use bdk::database::MemoryDatabase;
use bdk::electrum_client::Client;
use bdk::Wallet;
use clap::crate_version;
use clap::Clap;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

mod ur;
use ur::{is_ur_address, is_ur_descriptor, parse_ur_descriptor, psbt_as_ur};

mod errors;
use errors::SweepError;

#[derive(Serialize, Deserialize, Debug)]
struct Psbt {
    base64: String,
    ur: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct CliOutput {
    amount: u64,
    fees: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    address: Option<String>,
    timestamp: u64,
    txid: String,
    psbt: Psbt,
}

#[derive(Clap, Debug)]
#[clap(version=crate_version!())]
/// Sweep funds from a Bitcoin output descriptor
struct CliInput {
    /// Descriptor in UR format or in Bitcoin Core compatible format
    #[clap(short = 'd')]
    descriptor: String,
    /// Change descriptor in UR format or in Bitcoin core compatible format
    #[clap(short = 'c')]
    descriptor_chg: String,
    /// Address gap limit to search within for available funds
    #[clap(short = 'g')]
    address_gap_limit: Option<u32>,
    /// Bitcoin address in UR format or in Bitcoin Core compatible format.
    #[clap(short)]
    address: String,
    /// Target (number of blocks) used to estimate the fee rate for a PSBT
    #[clap(short, default_value = "6")]
    target: usize,
    /// Bitcoin network
    #[clap(short, default_value = "testnet", possible_values=&["mainnet", "testnet"])]
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

    let client = Client::new("ssl://electrum.blockstream.info:60002")?;

    let netw = if opt.network == "mainnet" {
        bdk::bitcoin::Network::Bitcoin
    } else {
        bdk::bitcoin::Network::Testnet
    };

    let wallet = Wallet::new(
        &descriptor,
        Some(&descriptor_chg),
        netw,
        MemoryDatabase::default(),
        ElectrumBlockchain::from(client),
    )?;

    let feerate = wallet.client().estimate_fee(opt.target)?;

    wallet.sync(noop_progress(), opt.address_gap_limit)?;

    let addr = if is_ur_address(opt.address.clone()) {
        return Err(SweepError::new(
            "cli arg".to_string(),
            "UR address not implemented".to_string(),
        ));
        //decode_ur_address(opt.address)?
    } else {
        Address::from_str(&opt.address)?
    };

    let (psbt, details) = {
        let mut builder = wallet.build_tx();
        builder.drain_wallet();
        builder
            .set_single_recipient(addr.script_pubkey())
            .enable_rbf()
            .fee_rate(feerate);
        builder.finish()?
    };

    let out = CliOutput {
        amount: details.sent,
        fees: details.fees,
        address: Some(addr.to_string()),
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
