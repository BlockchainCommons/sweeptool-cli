use assert_cmd::prelude::*; // Add methods on commands
use predicates::prelude::*; // Used for writing assertions
use serde_json::Value;
use std::process::Command;

const SWEEPTOOL: &str = "sweeptool";
const NIGIRI: &str = "nigiri";

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
//#[cfg_attr(not(feature = "nigiri"), ignore)]
fn h12() -> Result<(), Box<dyn std::error::Error>> {
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

    let mut nigiri = Command::new(NIGIRI);
    nigiri.arg("stop").arg("--delete").output().unwrap();

    let mut nigiri = Command::new(NIGIRI);
    nigiri.arg("start").output().unwrap();

    let mut nigiri = Command::new(NIGIRI);
    // Generate funds to Rx address 1:
    nigiri
        .arg("rpc")
        .arg("generatetoaddress")
        .arg("1")
        .arg("mqcxhkif3CQjmEWHGKJibMxRrNv8FKfnve")
        .output()
        .unwrap();

    println!("-- {:?}", nigiri.output());

    // Generate funds to Rx address 3:
    let mut nigiri = Command::new(NIGIRI);
    nigiri
        .arg("rpc")
        .arg("generatetoaddress")
        .arg("1")
        .arg("mvCntejWFwemnhSsCU51s7UKHqV37jn41V")
        .output()
        .unwrap();
    println!("-- {:?}", nigiri.output());

    // Generate funds to Chg address 0:
    let mut nigiri = Command::new(NIGIRI);
    nigiri
        .arg("rpc")
        .arg("generatetoaddress")
        .arg("1")
        .arg("mtDh4jQfAZg9DFnX6nirXLokjXN3tDtHUg")
        .output()
        .unwrap();

    // Generate funds to Chg address 2:
    let mut nigiri = Command::new(NIGIRI);
    nigiri
        .arg("rpc")
        .arg("generatetoaddress")
        .arg("1")
        .arg("mrqSutMAGBAont2XR3NY56VoY9QQRUAM2n")
        .output()
        .unwrap();

    //println!("++ {:?}", nigiri);

    thread::sleep(Duration::from_millis(1000));

    let mut cmd = Command::cargo_bin(SWEEPTOOL)?;

    let c="pkh([c258d2e4/44h/1h/0h]tpubD6NzVbkrYhZ4Yg9Rz1bXTTrc4TqZ8odbPaXrnrWX6cbDsXvH96FLDeRsckXohEkzGdAn5hbtK6iN7pCB1DeUpVwofEXCsN2StwWtU2SxE3f/0/*)";
    let d="pkh([c258d2e4/44h/1h/0h]tpubD6NzVbkrYhZ4Yg9Rz1bXTTrc4TqZ8odbPaXrnrWX6cbDsXvH96FLDeRsckXohEkzGdAn5hbtK6iN7pCB1DeUpVwofEXCsN2StwWtU2SxE3f/1/*)";
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
    println!("{:?}", out);

    let val: Value = serde_json::from_str(&String::from_utf8_lossy(&out.stdout))?;
    //Object(Map<String, Value>)

    if let Value::Object(m) = val {
        let psbt = m.get("psbt").unwrap();
        if let Value::Object(m) = psbt {
            let psbt = m.get("base64").unwrap();
            println!("*psbt: {:?}", psbt);

            use bdk::bitcoin::consensus::deserialize;
            use bdk::bitcoin::util::psbt::*;
            let psbt = psbt.as_str().unwrap();
            println!("*_psbt: {:?}", psbt);
            let mut psbt: PartiallySignedTransaction =
                bdk::bitcoin::consensus::deserialize(&base64::decode(psbt).unwrap()).unwrap();

            println!("inputs {:?}", psbt.inputs);
            println!("outputs {:?}", psbt.outputs);
        }
    }

    //cmd.assert()
    //    .success()
    //     .stdout(predicate::str::contains("USAGE"));

    //nigiri.arg("stop").arg("delete");
    Ok(())
}
