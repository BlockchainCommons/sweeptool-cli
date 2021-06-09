use assert_cmd::prelude::*; // Add methods on commands
use bdk::bitcoin::util::psbt::*;
use bdk::bitcoin::Address;
use bdk::bitcoin::Network;
use bdk::blockchain::noop_progress;
use bdk::blockchain::ElectrumBlockchain;
use bdk::database::MemoryDatabase;
use bdk::electrum_client::Client;
use bdk::Wallet;
use predicates::prelude::*; // Used for writing assertions
use serde_json::Value;
use std::process::Command;

const SWEEPTOOL: &str = "sweeptool";
const NIGIRI: &str = "nigiri";
const CLIENT_URL: &str = "127.0.0.1:51401";

#[test]
fn help_subcommand() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin(SWEEPTOOL)?;

    cmd.arg("-h");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("USAGE"));

    Ok(())
}

// run with: cargo test --features nigiri
#[test]
#[cfg_attr(not(feature = "nigiri"), ignore)]
fn test_sweeping() -> Result<(), Box<dyn std::error::Error>> {
    // Rx output descriptor:
    // "pkh([c258d2e4/44h/1h/0h]tpubD6NzVbkrYhZ4Yg9Rz1bXTTrc4TqZ8odbPaXrnrWX6cbDsXvH96FLDeRsckXohEkzGdAn5hbtK6iN7pCB1DeUpVwofEXCsN2StwWtU2SxE3f/0/*)"
    // 0. mn9qXHZsAQT6A1fkMvi5nmWmCzUEyLWZhv
    // 1. mqcxhkif3CQjmEWHGKJibMxRrNv8FKfnve
    // 2. mqFj6KcftBr8paK19gYRWt2PP5kZw3AhKb
    // 3. mvCntejWFwemnhSsCU51s7UKHqV37jn41V
    // 4. msZrw3N91LhHeH1ZkKXiGTnyquZ7Z9aaMn

    // Chg output descriptor:
    // "pkh([c258d2e4/44h/1h/0h]tpubD6NzVbkrYhZ4Yg9Rz1bXTTrc4TqZ8odbPaXrnrWX6cbDsXvH96FLDeRsckXohEkzGdAn5hbtK6iN7pCB1DeUpVwofEXCsN2StwWtU2SxE3f/1/*)"
    // 0. mtDh4jQfAZg9DFnX6nirXLokjXN3tDtHUg
    // 1. mw9c4zBpKbJJTDeYG5w1oC7WCmEMRJ4vyQ
    // 2. mrqSutMAGBAont2XR3NY56VoY9QQRUAM2n
    // 3. mp146R4cufJRBjyyg9hgbJ2EZSpFr6wS1U
    // 4. mifMkh8AWgCUoA8UYz9iKz5PyD5bcJNpnP

    // destination output descriptor
    // wpkh([c258d2e4/84h/1h/1h]tpubDDYkZojQFQjht8Tm4jsS3iuEmKjTiEGjG6KnuFNKKJb5A6ZUCUZKdvLdSDWofKi4ToRCwb9poe1XdqfUnP4jaJjCB2Zwv11ZLgSbnZSNecE/0/*)
    // 0. bcrt1qzg4mckdh50nwdm9hkzq06528rsu73hjxytqkxs
    // 1. bcrt1qvctwrh8ckrex8daxya4xleaevcp299tt0v37w9
    // 2. bcrt1qru09gqacyezszqgweakmawr3znsp23n5dyy0fu
    // 3. bcrt1qkn7qfsmdxgktpnquhq8q6nmvastac2dw2cxmrd
    // 4. bcrt1qqhkxlnr452e9zrkpzq5h4cw38dcntry98dnn89

    // destination change descriptor
    // wpkh([c258d2e4/84h/1h/1h]tpubDDYkZojQFQjht8Tm4jsS3iuEmKjTiEGjG6KnuFNKKJb5A6ZUCUZKdvLdSDWofKi4ToRCwb9poe1XdqfUnP4jaJjCB2Zwv11ZLgSbnZSNecE/1/*)
    // 0. bcrt1qpqmnwemer994g7lguut207cxm4dry8p5a2c3hc
    // 1. bcrt1q763lyaz775cnd86mf7v59t4589fpejfry83z0r
    // 2. bcrt1q5v2fh8ne73ysc0cx8ynuh474hy2vauz2r8zvnt
    // 3. bcrt1qasqk6uhemj8mr7xv9ktxfe6jsntkgkpcnka7lf
    // 4. bcrt1qf8zxuslvxhrfkew8j5frusc635gn6e39x2m6mm

    use std::thread;
    use std::time::Duration;

    // An address used to generate blocks and dump the reward
    const ADDRESS_UNRELATED: &str = "2N3oefVeg6stiTb5Kh3ozCSkaqmx91FDbsm";

    // genreates blocks over a halving period
    fn generate_blocks() {
        let mut nigiri = Command::new(NIGIRI);
        // Generate blocks to an unrelated address to trigger the halving
        nigiri
            .arg("rpc")
            .arg("generatetoaddress")
            .arg("150")
            .arg(ADDRESS_UNRELATED)
            .output()
            .unwrap();
    }

    let mut nigiri = Command::new(NIGIRI);
    nigiri.arg("stop").arg("--delete").output().unwrap();

    let mut nigiri = Command::new(NIGIRI);
    nigiri.arg("start").output().unwrap();

    let mut nigiri = Command::new(NIGIRI);
    // Generate 50 BTC to Rx address 1:
    nigiri
        .arg("rpc")
        .arg("generatetoaddress")
        .arg("1")
        .arg("mqcxhkif3CQjmEWHGKJibMxRrNv8FKfnve")
        .output()
        .unwrap();

    generate_blocks();

    // Generate 25 BTC to Rx address 3:
    let mut nigiri = Command::new(NIGIRI);
    nigiri
        .arg("rpc")
        .arg("generatetoaddress")
        .arg("1")
        .arg("mvCntejWFwemnhSsCU51s7UKHqV37jn41V")
        .output()
        .unwrap();
    //println!("-- {:?}", nigiri.output());

    generate_blocks();

    // Generate 12.5BTC to Chg address 0:
    let mut nigiri = Command::new(NIGIRI);
    nigiri
        .arg("rpc")
        .arg("generatetoaddress")
        .arg("1")
        .arg("mtDh4jQfAZg9DFnX6nirXLokjXN3tDtHUg")
        .output()
        .unwrap();

    generate_blocks();

    // Generate ~6BTC to Chg address 2:
    let mut nigiri = Command::new(NIGIRI);
    nigiri
        .arg("rpc")
        .arg("generatetoaddress")
        .arg("1")
        .arg("mrqSutMAGBAont2XR3NY56VoY9QQRUAM2n")
        .output()
        .unwrap();

    thread::sleep(Duration::from_millis(1000));

    // TEST CASE:
    let mut cmd = Command::cargo_bin(SWEEPTOOL)?;

    let c="pkh([c258d2e4/44h/1h/0h]tpubD6NzVbkrYhZ4Yg9Rz1bXTTrc4TqZ8odbPaXrnrWX6cbDsXvH96FLDeRsckXohEkzGdAn5hbtK6iN7pCB1DeUpVwofEXCsN2StwWtU2SxE3f/1/*)";
    let d="pkh([c258d2e4/44h/1h/0h]tpubD6NzVbkrYhZ4Yg9Rz1bXTTrc4TqZ8odbPaXrnrWX6cbDsXvH96FLDeRsckXohEkzGdAn5hbtK6iN7pCB1DeUpVwofEXCsN2StwWtU2SxE3f/0/*)";
    let e="wpkh([c258d2e4/84h/1h/1h]tpubDDYkZojQFQjht8Tm4jsS3iuEmKjTiEGjG6KnuFNKKJb5A6ZUCUZKdvLdSDWofKi4ToRCwb9poe1XdqfUnP4jaJjCB2Zwv11ZLgSbnZSNecE/0/*)";
    let s="wpkh([c258d2e4/84h/1h/1h]tpubDDYkZojQFQjht8Tm4jsS3iuEmKjTiEGjG6KnuFNKKJb5A6ZUCUZKdvLdSDWofKi4ToRCwb9poe1XdqfUnP4jaJjCB2Zwv11ZLgSbnZSNecE/1/*)";

    cmd.arg("-d")
        .arg(d)
        .arg("-c")
        .arg(c)
        .arg("-e")
        .arg(e)
        .arg("-s")
        .arg(s)
        .arg("-n")
        .arg("regtest")
        .output()
        .unwrap();

    let out = cmd.output().unwrap();
    let val: Value = serde_json::from_str(&String::from_utf8_lossy(&out.stdout))?;

    if let Value::Object(m) = val {
        let psbt = m.get("psbt").unwrap();
        if let Value::Object(m) = psbt {
            let psbt = m.get("base64").unwrap();

            let psbt = psbt.as_str().unwrap();
            let psbt: PartiallySignedTransaction =
                bdk::bitcoin::consensus::deserialize(&base64::decode(psbt).unwrap()).unwrap();

            let tx = psbt.clone().extract_tx();

            // No additional change addresses must be created:
            assert_eq!(tx.output.len(), 4);

            for i in 0..tx.output.len() {
                let address = Address::from_script(
                    &tx.output[i].script_pubkey,
                    bdk::bitcoin::Network::Regtest,
                )
                .unwrap();

                // Here we are checking if the amounts are correctly sent to addresses of destination
                // output descriptors which match the source output descriptor by its type (Rcv or Chg) and index.
                if tx.output[i].value > 2_500_000_000 {
                    // 50 BTC (minus fees) from Rx address 1 of the source output descriptor must map
                    // to Rx address 1 of the destination output descriptor
                    assert_eq!(
                        "bcrt1qvctwrh8ckrex8daxya4xleaevcp299tt0v37w9",
                        address.to_string()
                    );
                } else if tx.output[i].value > 1_250_000_000 {
                    assert_eq!(
                        "bcrt1qkn7qfsmdxgktpnquhq8q6nmvastac2dw2cxmrd",
                        address.to_string()
                    );
                } else if tx.output[i].value > 625_000_000 {
                    assert_eq!(
                        "bcrt1qpqmnwemer994g7lguut207cxm4dry8p5a2c3hc",
                        address.to_string()
                    );
                } else {
                    assert_eq!(
                        "bcrt1q5v2fh8ne73ysc0cx8ynuh474hy2vauz2r8zvnt",
                        address.to_string()
                    );
                }
            }
        }
    }

    // TEST CASE: Let's test the same CMD with a smaller address gap limit of 1:
    let mut cmd = Command::cargo_bin(SWEEPTOOL)?;

    let c="pkh([c258d2e4/44h/1h/0h]tpubD6NzVbkrYhZ4Yg9Rz1bXTTrc4TqZ8odbPaXrnrWX6cbDsXvH96FLDeRsckXohEkzGdAn5hbtK6iN7pCB1DeUpVwofEXCsN2StwWtU2SxE3f/1/*)";
    let d="pkh([c258d2e4/44h/1h/0h]tpubD6NzVbkrYhZ4Yg9Rz1bXTTrc4TqZ8odbPaXrnrWX6cbDsXvH96FLDeRsckXohEkzGdAn5hbtK6iN7pCB1DeUpVwofEXCsN2StwWtU2SxE3f/0/*)";
    let e="wpkh([c258d2e4/84h/1h/1h]tpubDDYkZojQFQjht8Tm4jsS3iuEmKjTiEGjG6KnuFNKKJb5A6ZUCUZKdvLdSDWofKi4ToRCwb9poe1XdqfUnP4jaJjCB2Zwv11ZLgSbnZSNecE/0/*)";
    let s="wpkh([c258d2e4/84h/1h/1h]tpubDDYkZojQFQjht8Tm4jsS3iuEmKjTiEGjG6KnuFNKKJb5A6ZUCUZKdvLdSDWofKi4ToRCwb9poe1XdqfUnP4jaJjCB2Zwv11ZLgSbnZSNecE/1/*)";

    cmd.arg("-d")
        .arg(d)
        .arg("-c")
        .arg(c)
        .arg("-e")
        .arg(e)
        .arg("-s")
        .arg(s)
        .arg("-n")
        .arg("regtest")
        .arg("-g")
        .arg("1")
        .output()
        .unwrap();

    // With an address gap limit=1 we should register only 12.5BTC on Chg address 0:
    cmd.assert()
        .success()
        .stdout(predicate::str::contains(r#""amount":1250000000"#));

    // Let's parse the PSBT:
    let out = cmd.output().unwrap();
    let val: Value = serde_json::from_str(&String::from_utf8_lossy(&out.stdout))?;

    if let Value::Object(m) = val {
        let psbt = m.get("psbt").unwrap();
        if let Value::Object(m) = psbt {
            let psbt = m.get("base64").unwrap();

            let psbt = psbt.as_str().unwrap();
            let psbt: PartiallySignedTransaction =
                bdk::bitcoin::consensus::deserialize(&base64::decode(psbt).unwrap()).unwrap();

            let tx = psbt.clone().extract_tx();

            // We must have exactly 1 destination address
            assert_eq!(tx.output.len(), 1);

            for i in 0..tx.output.len() {
                let address = Address::from_script(
                    &tx.output[i].script_pubkey,
                    bdk::bitcoin::Network::Regtest,
                )
                .unwrap();

                // The amount sent to the destination address should be a little lower than 1_250_000_000
                if tx.output[i].value < 1_250_000_000 && tx.output[i].value > 625_000_000 {
                    // The receiving address should be of index 0 on the destination output
                    // descriptor for Change purpose (Chg)
                    assert_eq!(
                        "bcrt1qpqmnwemer994g7lguut207cxm4dry8p5a2c3hc",
                        address.to_string()
                    );
                }
            }
        }
    }

    // TEST CASE: let's sweep the funds to some other destination (Rcv) output descriptor in UR format
    let mut cmd = Command::cargo_bin(SWEEPTOOL)?;

    // source: https://github.com/BlockchainCommons/Research/blob/master/papers/bcr-2020-010-output-desc.md#exampletest-vector-3
    let e_core_format = "sh(multi(2,022f01e5e15cca351daff3843fb70f3c2f0a1bdd05e5af888a67784ef3e10a2a01,03acd484e2f0c7f65309ad178a9f559abde09796974c57e714c35f110dfc27ccbe))";
    let e_ur_format= "ur:crypto-output/taadmhtaadmtoeadaoaolftaadeyoyaxhdclaodladvwvyhhsgeccapewflrfhrlbsfndlbkcwutahvwpeloleioksglwfvybkdradtaadeyoyaxhdclaxpstylrvowtstynguaspmchlenegonyryvtmsmtmsgshgvdbbsrhebybtztdisfrnpfadremh";
    // Let's first derive a few addresses from this descriptor so we can test against them later
    // We are expecting ~50 BTC on Rx address 1 and ~25 BTC on Rx address 3:
    use bdk::wallet::AddressIndex::New;

    let client = Client::new(CLIENT_URL)?;

    let wallet_destination = Wallet::new_offline(
        e_core_format,
        Some(s),
        Network::Regtest,
        MemoryDatabase::default(),
    )?;

    let _addr_0 = wallet_destination.get_address(New)?;
    let addr_1 = wallet_destination.get_address(New)?;
    let _addr_2 = wallet_destination.get_address(New)?;
    let addr_3 = wallet_destination.get_address(New)?;

    let wallet_origin = Wallet::new(
        c,
        Some(d),
        Network::Regtest,
        MemoryDatabase::default(),
        ElectrumBlockchain::from(client),
    )?;

    cmd.arg("-d")
        .arg(d)
        .arg("-c")
        .arg(c)
        .arg("-e")
        .arg(e_ur_format)
        .arg("-s")
        .arg(s)
        .arg("-n")
        .arg("regtest")
        .output()
        .unwrap();

    let out = cmd.output().unwrap();
    let val: Value = serde_json::from_str(&String::from_utf8_lossy(&out.stdout))?;

    if let Value::Object(m) = val {
        let psbt = m.get("psbt").unwrap();
        if let Value::Object(m) = psbt {
            let psbt = m.get("base64").unwrap();

            use bdk::bitcoin::util::psbt::*;
            let psbt = psbt.as_str().unwrap();
            let psbt: PartiallySignedTransaction =
                bdk::bitcoin::consensus::deserialize(&base64::decode(psbt).unwrap()).unwrap();

            let tx = psbt.clone().extract_tx();

            // No additional change addresses must be created:
            assert_eq!(tx.output.len(), 4);

            for i in 0..tx.output.len() {
                let address = Address::from_script(
                    &tx.output[i].script_pubkey,
                    bdk::bitcoin::Network::Regtest,
                )
                .unwrap();

                // Here we are checking if the amounts are correctly sent to addresses of destination
                // output descriptors which match the source output descriptor by its type (Rcv or Chg) and index.
                if tx.output[i].value > 2_500_000_000 {
                    // 50 BTC (minus fees) from Rx address 1 of the source output descriptor must map
                    // to Rx address 1 of the destination output descriptor
                    assert_eq!(addr_1.to_string(), address.to_string());
                } else if tx.output[i].value > 1_250_000_000 {
                    assert_eq!(addr_3.to_string(), address.to_string());
                } else if tx.output[i].value > 625_000_000 {
                    assert_eq!(
                        "bcrt1qpqmnwemer994g7lguut207cxm4dry8p5a2c3hc",
                        address.to_string()
                    );
                } else {
                    assert_eq!(
                        "bcrt1q5v2fh8ne73ysc0cx8ynuh474hy2vauz2r8zvnt",
                        address.to_string()
                    );
                }
            }
        }
    }

    // TEST CASE: sweep to an address
    let mut cmd = Command::cargo_bin(SWEEPTOOL)?;

    let addr = "2NA2wt6vsNpENreZEydjevbuvg81v6Mej26";

    cmd.arg("-d")
        .arg(d)
        .arg("-c")
        .arg(c)
        .arg("-a")
        .arg(addr)
        .arg("-n")
        .arg("regtest")
        .output()
        .unwrap();

    wallet_origin.sync(noop_progress(), None)?;

    let out = cmd.output().unwrap();
    let val: Value = serde_json::from_str(&String::from_utf8_lossy(&out.stdout))?;

    if let Value::Object(m) = val {
        let psbt = m.get("psbt").unwrap();
        if let Value::Object(m) = psbt {
            let psbt = m.get("base64").unwrap();

            let psbt = psbt.as_str().unwrap();
            let psbt: PartiallySignedTransaction =
                bdk::bitcoin::consensus::deserialize(&base64::decode(psbt).unwrap()).unwrap();

            let tx = psbt.clone().extract_tx();

            assert_eq!(tx.output.len(), 1);

            let address =
                Address::from_script(&tx.output[0].script_pubkey, bdk::bitcoin::Network::Regtest)
                    .unwrap();

            assert_eq!(addr, address.to_string());

            let source_amount = wallet_origin.get_balance().unwrap();
            let destination_amount = tx.output[0].value;

            let fees = source_amount - destination_amount;

            cmd.assert()
                .success()
                .stdout(predicate::str::contains(format!(
                    "{}{}",
                    r#""fees":"#, fees
                )));
        } else {
            panic!("output error");
        }
    }

    Ok(())
}
