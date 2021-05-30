use bdk::bitcoin::consensus::serialize;
use bdk::bitcoin::Address;
use bdk::blockchain::{noop_progress, ElectrumBlockchain};
use bdk::database::MemoryDatabase;
use bdk::electrum_client::Client;
use bdk::FeeRate;
use bdk::Wallet;
use clap::Clap;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

mod ur;
use ur::{decode_ur_address, is_ur_address, psbt_as_ur};

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
#[clap()]
/// Sweeptool-cli
struct CliInput {
    /// Descriptor in UR format or in bitcoin core compatible format
    #[clap(short = 'd')]
    descriptor: Option<String>,
    #[clap(short = 'g')]
    address_gap_limit: Option<u32>,
    #[clap(short)]
    address: String,
}

fn main() -> Result<(), bdk::Error> {
    /*
        let address_ur =
            "ur:crypto-address/oyaxghktrswzbnhnvwcpurpkeogdsrndaxbkhlaegllsnyolrsemgu".to_string();

        //let address_ur_ethereum = "ur:crypto-address/oeadtaadehoeadcsfnaoadaxghlyrlvtmyihryykielnamspnlmkptsflyieeskofllosfeecf".to_owned();

        let address_ur = "ur:crypto-address/oeahtaadehoyaoadaxhfaebbvenlpddrnydpdlkbrtlplffdtsgueektdrpfuodlbyonsbdw".to_string();

        let address_ur = "ur:crypto-address/oeahtaadehoyaoadaxhfaebbdydwahwkvtdewttsdnkshflgykrdmdhsfycwlyatfytabzia".to_string();

        decode_ur_address(address_ur);

    */
    let opt = CliInput::parse();

    // let descriptor = opt.descriptor.unwrap();

    let descriptor = if let Some(desc) = opt.descriptor {
        desc
    } else {
        panic!("UR descriptor cannot be currently passed via STDIN. Pass it as an CLI arg")
    };

    // create a change descriptor TODO: make a function   NOT OK!
    let mut descriptor_chg = descriptor.clone();
    descriptor_chg.pop();
    descriptor_chg.pop();
    descriptor_chg.pop();
    descriptor_chg.pop();
    descriptor_chg.push('1');
    descriptor_chg.push('/');
    descriptor_chg.push('*');
    descriptor_chg.push(')');

    let client = Client::new("ssl://electrum.blockstream.info:60002")?;
    let wallet = Wallet::new(
        &descriptor,
        Some(&descriptor_chg),
        bdk::bitcoin::Network::Testnet,
        MemoryDatabase::default(),
        ElectrumBlockchain::from(client),
    )?;

    wallet.sync(noop_progress(), opt.address_gap_limit)?;

    let addr = if is_ur_address(opt.address.clone()) {
        decode_ur_address(opt.address)
    } else {
        Address::from_str(&opt.address).unwrap()
    };

    let (psbt, details) = {
        let mut builder = wallet.build_tx();
        builder.drain_wallet();
        builder
            .set_single_recipient(addr.script_pubkey())
            .enable_rbf()
            .fee_rate(FeeRate::from_sat_per_vb(5.0)); // TODO lookup for optimal fee
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
            ur: psbt_as_ur(serialize(&psbt)),
        },
    };

    println!("{}", serde_json::to_string(&out)?);

    Ok(())
}
