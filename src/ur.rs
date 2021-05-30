use bdk::bitcoin::hashes::Hash;
use serde::{Deserialize, Serialize};
use serde_cbor::value::Value;
use std::collections::HashMap;
use std::convert::TryInto;
use std::str::FromStr;
use ur_rs::bytewords;

use serde_cbor::tags::Tagged;

pub fn psbt_as_ur(psbt: Vec<u8>) -> String {
    use serde_cbor::to_vec;
    let arr = Value::Bytes(psbt.clone());
    let psbt_ = to_vec(&arr).unwrap();
    let bytewrds = bytewords::encode(&psbt_, &bytewords::Style::Minimal);
    let psbt_ur = "ur:crypto-psbt/".to_owned() + &bytewrds;
    psbt_ur
}

#[derive(Debug, PartialEq, Deserialize)]
pub enum CborNetwork {
    Mainnet = 0,
    Testnet = 1,
}

use std::convert::TryFrom;

#[derive(Debug, PartialEq, Deserialize)]
pub struct HDKey<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    offset: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_master: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_private: Option<bool>,
    pub key_data: &'a [u8],
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chain_code: Option<&'a [u8]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_info: Option<CryptoCoinInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub origin: Option<CryptoKeyPath>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<CryptoKeyPath2>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_fingerprint: Option<u32>,
    #[serde(skip)]
    name: Option<String>,
    #[serde(skip)]
    note: Option<String>,
}

#[derive(Debug, PartialEq, Serialize)]
pub struct CPsbt {
    pub data: Vec<u8>,
}

#[derive(Debug, PartialEq)]
pub struct CryptoKeyPath2 {
    // this is only for the tail part of the output descriptor
    // which can contain e.g. /1/*
    pub components: String,
}

impl<'a, 'de> Deserialize<'de> for CryptoKeyPath2 {
    fn deserialize<D: serde::de::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let tagged = Value::deserialize(deserializer)?;
        let mut obj = CryptoKeyPath2 {
            components: "".to_string(),
        };

        // TODO: uint31 = uint32 .lt 0x80000000
        if let Value::Tag(_number, val_nxt) = tagged {
            // TODO assert number
            if let Value::Map(m) = *val_nxt.clone() {
                let arr = m.get(&Value::Integer(1)).unwrap_or(&Value::Integer(0)); // this will skip parsing array in the next step
                if let Value::Array(a) = arr {
                    for i in 0..a.len() - 1 {
                        if let Value::Integer(ar) = a[i] {
                            obj.components.push_str(&format!("/{}", ar));
                            if a[i + 1] == Value::Bool(true) {
                                obj.components.push('h');
                            };
                        } else if let Value::Array(_val) = &a[i] {
                            obj.components.push_str("/*");
                            if a[i + 1] == Value::Bool(true) {
                                obj.components.push('h');
                            };
                        }
                    }
                }
            }
        }
        Ok(obj)
    }
}

#[derive(Debug, PartialEq)]
pub struct CryptoKeyPath {
    pub components: Vec<bdk::bitcoin::util::bip32::ChildNumber>,
    pub source_fingerprint: u32,
    pub depth: u8,
    pub components_str: String,
}

impl Serialize for CryptoKeyPath {
    fn serialize<S: serde::ser::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        Tagged::new(Some(304), &self).serialize(s)
    }
}

impl<'a, 'de> Deserialize<'de> for CryptoKeyPath {
    fn deserialize<D: serde::de::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let tagged = Value::deserialize(deserializer)?;
        let mut obj = CryptoKeyPath {
            components: Vec::new(),
            source_fingerprint: 0,
            depth: 0,
            components_str: "".to_string(),
        };

        // TODO: uint31 = uint32 .lt 0x80000000
        if let Value::Tag(_number, val_nxt) = tagged {
            // TODO assert number
            if let Value::Map(m) = *val_nxt.clone() {
                let arr = m.get(&Value::Integer(1)).unwrap_or(&Value::Integer(0)); // this will skip parsing array in the next step TODO!!! solve unwrap
                if let Value::Array(a) = arr {
                    for i in 0..a.len() - 1 {
                        // TODO 0 to -1 if len == 0 error
                        if i == 0 {
                            if obj.source_fingerprint != 0 {
                                obj.components_str
                                    .push_str(&format!("[{:08x}", obj.source_fingerprint));
                            } else {
                                obj.components_str.push_str("[m");
                            }
                        }
                        if let Value::Integer(ar) = a[i] {
                            obj.components_str.push_str(&format!("/{}", ar));
                            let indx = if a[i + 1] == Value::Bool(true) {
                                obj.components_str.push('h');
                                bdk::bitcoin::util::bip32::ChildNumber::from_hardened_idx(
                                    ar.try_into().unwrap(),
                                )
                                .unwrap()
                            } else {
                                bdk::bitcoin::util::bip32::ChildNumber::from_normal_idx(
                                    ar.try_into().unwrap(),
                                )
                                .unwrap()
                            };
                            obj.components.push(indx);
                        }

                        if i == a.len() - 1 - 1 {
                            obj.components_str.push_str("]");
                        }
                    }
                    let source_fingerprint = m.get(&Value::Integer(2));
                    if let Some(Value::Integer(s)) = source_fingerprint {
                        obj.source_fingerprint = *s as u32;
                    }
                    let depth = m.get(&Value::Integer(3));
                    if let Some(Value::Integer(s)) = depth {
                        // depth always takes precedense over components length
                        obj.depth = *s as u8;
                        assert!(obj.depth >= obj.components.len() as u8);
                    } else {
                        obj.depth = obj.components.len() as u8;
                    }
                }
            }
        }
        Ok(obj)
    }
}

#[derive(Debug, PartialEq)]
struct Multisig {
    threshold: u32,
    keys: Vec<EcKey>,
}

impl Serialize for Multisig {
    fn serialize<S: serde::ser::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        Tagged::new(Some(406), &self).serialize(s)
    }
}

#[derive(Debug, PartialEq)]
pub struct EcKey {
    pub curve: Option<u32>, // Must be 0 for BTC or omitted
    pub is_private: Option<bool>,
    pub data: Vec<u8>,
}

impl<'a> Deserialize<'a> for EcKey {
    fn deserialize<D: serde::de::Deserializer<'a>>(deserializer: D) -> Result<Self, D::Error> {
        println!("HERE!");
        // spec: https://github.com/BlockchainCommons/Research/blob/master/papers/bcr-2020-008-eckey.md
        // Note: hashmap could have bytestrings as values, but here integer works according to cryptoconinfo spec
        //let tagged = Tagged::<HashMap<u8, &[u8]>>::deserialize(deserializer)?;
        let tagged = Value::deserialize(deserializer)?;

        let mut obj = EcKey {
            curve: None,
            is_private: None,
            data: Vec::new(),
        };

        if let Value::Tag(_number, val_nxt) = tagged {
            if let Value::Map(m) = *val_nxt.clone() {
                // TODO check for 1 and 2 integers
                let data = m.get(&Value::Integer(3)).unwrap();
                if let Value::Bytes(b) = data.clone() {
                    obj.data = b;
                }
            }
            Ok(obj)
        } else {
            Ok(obj)
        }

        /*
        match tagged.tag {
            Some(306) | None => {
                if let Some(curve) = tagged.value.get(&1) {
                    let num = u32::from_be_bytes(*pop(*curve));
                    obj.curve = Some(num);
                }

                if let Some(is_private) = tagged.value.get(&2) {
                    let is_private: bool = serde_cbor::de::from_slice(&is_private).unwrap();
                    obj.is_private = Some(is_private);
                }

                if let Some(data) = tagged.value.get(&3) {
                    println!("data {:?}", data);
                    //let data: &[u8] = serde_cbor::de::from_slice(&data).unwrap();
                    obj.data = data;
                }

                println!("* * * {:?}", tagged.value);

                Ok(obj)
            }
            Some(_) => Err(serde::de::Error::custom("unexpected tag")),
        } */
    }
}

impl TryFrom<u32> for CborNetwork {
    type Error = ();
    fn try_from(v: u32) -> Result<Self, Self::Error> {
        match v {
            x if x == CborNetwork::Mainnet as u32 => Ok(CborNetwork::Mainnet),
            x if x == CborNetwork::Testnet as u32 => Ok(CborNetwork::Testnet),
            _ => Err(()),
        }
    }
}

impl TryFrom<CborNetwork> for bdk::bitcoin::Network {
    type Error = ();
    fn try_from(v: CborNetwork) -> Result<Self, Self::Error> {
        match v {
            CborNetwork::Mainnet => Ok(bdk::bitcoin::Network::Bitcoin),
            CborNetwork::Testnet => Ok(bdk::bitcoin::Network::Testnet),
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct CryptoCoinInfo {
    //#[serde(skip_serializing_if = "Option::is_none")]
    pub type_: Option<u32>, // Must be 0 for BTC or omitted
    //#[serde(skip_serializing_if = "Option::is_none")]
    pub network: Option<CborNetwork>,
}

impl Serialize for CryptoCoinInfo {
    fn serialize<S: serde::ser::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        Tagged::new(Some(305), &self).serialize(s)
    }
}

impl<'de> Deserialize<'de> for CryptoCoinInfo {
    fn deserialize<D: serde::de::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        // spec: https://github.com/BlockchainCommons/Research/blob/master/papers/bcr-2020-009-address.md
        // Note: hashmap could have bytestrings as values, but here integer works according to cryptoconinfo spec
        let tagged = Tagged::<HashMap<u8, u32>>::deserialize(deserializer)?;

        let mut obj = CryptoCoinInfo {
            type_: None,
            network: Some(CborNetwork::Mainnet),
        };

        match tagged.tag {
            Some(305) | None => {
                let type_val = tagged.value.get(&1);
                if type_val != None {
                    obj.type_ = Some(*type_val.unwrap());
                }

                let network_val = tagged.value.get(&2);
                if network_val != None {
                    let network = *network_val.unwrap();
                    obj.network = Some(network.try_into().unwrap());
                }
                Ok(obj)
            }
            Some(_) => Err(serde::de::Error::custom("unexpected tag")),
        }
    }
}

#[derive(Debug, PartialEq, Deserialize)]
pub struct CborAddress<'a> {
    /// This variable is just to address the offset difference.
    /// Namely the Blockchain Commons starts counting from 1, whereas this lib from 0
    #[serde(skip_serializing_if = "Option::is_none")]
    offset: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    // TODO: https://github.com/pyfisch/cbor/blob/master/examples/tags.rs
    info: Option<CryptoCoinInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    type_: Option<u32>,
    data: &'a [u8],
}

pub fn is_ur_address(ur: String) -> bool {
    ur.contains("ur:crypto-address")
}

pub fn decode_ur_address(ur: String) -> bdk::bitcoin::Address {
    let (_key, val) = ur.split_once(':').unwrap(); // TODO check key == ur
    let (_key, val) = val.split_once('/').unwrap(); // TODO check key == crypto-address
    let mut cbor = bytewords::decode(&val, &bytewords::Style::Minimal).unwrap();
    let cbor: CborAddress = serde_cbor::de::from_mut_slice(&mut cbor[..]).unwrap();
    let data = cbor.data.to_vec(); // pubkeyhash

    //println!("**data: {:?}", data);

    let network = if let Some(info) = cbor.info {
        if let Some(n) = info.network {
            match n {
                CborNetwork::Mainnet => bdk::bitcoin::Network::Bitcoin,
                _ => bdk::bitcoin::Network::Testnet,
            }
        } else {
            bdk::bitcoin::Network::Bitcoin
        }
    } else {
        bdk::bitcoin::Network::Bitcoin
    };

    // Try all possible payloads:
    /*
        // pkh
        let tmp1 = bdk::bitcoin::util::address::Payload::PubkeyHash(
            bdk::bitcoin::hash_types::PubkeyHash::from_slice(&data).unwrap(),
        );

        // p2sh
        let tmp2 = bdk::bitcoin::util::address::Payload::ScriptHash(
            bdk::bitcoin::hash_types::ScriptHash::from_slice(&data).unwrap(),
        );
    */

    bdk::bitcoin::Address {
        payload: bdk::bitcoin::util::address::Payload::PubkeyHash(
            bdk::bitcoin::hash_types::PubkeyHash::from_slice(&data).unwrap(),
        ),
        network: network,
    }
}

pub fn parse_ur_desc(val: Value, out: &mut String) -> Option<Box<Value>> {
    // first check if value is Nil
    if let Value::Tag(number, mut val_nxt) = val.clone() {
        match number {
            303 => {
                let p = serde_cbor::to_vec(&val_nxt).unwrap();
                let hdkey: HDKey = serde_cbor::de::from_slice(&p[..]).unwrap();
                println!("debug: hdkey: {:?}", hdkey);

                // TODO check if this is master key-> no need for dealing with with derivpath if yes
                // TODO implement iterators to use and_then
                let net = if let Some(info) = hdkey.use_info {
                    if let Some(n) = info.network {
                        n
                    } else {
                        CborNetwork::Mainnet
                    }
                } else {
                    CborNetwork::Mainnet
                };

                let keydata = &hdkey.key_data[..].to_vec();

                let childnumber = if let Some(ref origin) = hdkey.origin {
                    *origin
                        .components
                        .last()
                        .unwrap_or(&bdk::bitcoin::util::bip32::ChildNumber::from(0))
                } else {
                    bdk::bitcoin::util::bip32::ChildNumber::from(0)
                };

                let depth = if let Some(ref d) = hdkey.origin {
                    d.depth
                } else {
                    0
                };

                let xpub = bdk::bitcoin::util::bip32::ExtendedPubKey {
                    network: bdk::bitcoin::Network::try_from(net).unwrap(), // TODO
                    depth: depth,
                    parent_fingerprint: bdk::bitcoin::util::bip32::Fingerprint::from(
                        &hdkey.parent_fingerprint.unwrap_or(0).to_be_bytes()[..],
                    ),
                    child_number: childnumber,
                    public_key: bdk::bitcoin::PublicKey::from_slice(&keydata[..]).unwrap(),
                    chain_code: bdk::bitcoin::util::bip32::ChainCode::from(
                        &hdkey.chain_code.unwrap()[..],
                    ),
                };

                println!("debug xpub>>: {:?}", xpub);

                if let Some(c) = hdkey.origin {
                    out.push_str(&c.components_str);
                };

                out.push_str(&format!("{}", xpub.to_string()));

                if let Some(c) = hdkey.children {
                    out.push_str(&c.components);
                };
            }
            306 => {
                let p = serde_cbor::to_vec(&val).unwrap();
                let eckey: EcKey = serde_cbor::de::from_slice(&p).unwrap();
                out.push_str(&hex::encode(eckey.data));
            }
            400 => {
                out.push_str(&"sh(".to_string());
                val_nxt = parse_ur_desc(*val_nxt, out).unwrap();
                out.push_str(&")".to_string());
            }
            403 => {
                out.push_str(&"pkh(".to_string());
                // recursion here: parse_ur_desc
                val_nxt = parse_ur_desc(*val_nxt, out).unwrap();
                out.push_str(&")".to_string());
            }
            401 => {
                //println!("witness_public_key_hash");
                out.push_str(&"wsh(".to_string());
                val_nxt = parse_ur_desc(*val_nxt, out).unwrap();
                out.push_str(&")".to_string());
            }
            404 => {
                out.push_str(&"wpkh(".to_string());
                val_nxt = parse_ur_desc(*val_nxt, out).unwrap();
                out.push_str(&")".to_string());
            }
            406 | 407 => {
                if number == 406 {
                    out.push_str(&"multi(".to_string());
                } else {
                    out.push_str(&"sortedmulti(".to_string());
                }

                if let Value::Map(v) = *val_nxt.clone() {
                    let threshold = v.get(&Value::Integer(1)).unwrap();
                    if let Value::Integer(i) = threshold {
                        out.push_str(&format!("{},", i));
                    }
                    let arr = v.get(&Value::Integer(2)).unwrap();
                    if let Value::Array(v) = arr {
                        for i in v {
                            if let Value::Tag(num, _) = i {
                                if *num == 303 {
                                    // hdkey
                                    val_nxt = parse_ur_desc(i.clone(), out).unwrap();
                                } else if *num == 306 {
                                    // eckey
                                    let p = serde_cbor::to_vec(&i).unwrap();
                                    let eckey: EcKey = serde_cbor::de::from_slice(&p).unwrap(); // TODO
                                    out.push_str(&hex::encode(eckey.data));
                                }
                                out.push_str(",");
                            }
                        }
                        out.pop();
                    }
                }
                out.push_str(&")".to_string());
            }

            _ => panic!("wrong tag {:?}", number),
        }
        Some(val_nxt)
    } else {
        println!("false"); // TODO error out
        None
    }
}

#[test]
fn outputdesc_test_vector_5() -> Result<(), Box<dyn std::error::Error>> {
    let  inp = hex::decode("d90191d90196a201010282d9012fa403582103cbcaa9c98c877a26977d00825c956a238e8dddfbd322cce4f74b0b5bd6ace4a704582060499f801b896d83179a4374aeb7822aaeaceaa0db1f85ee3e904c4defbd968906d90130a1030007d90130a1018601f400f480f4d9012fa403582102fc9e5af0ac8d9b3cecfe2a888e2117ba3d089d8585886c9c826b6b22a98d12ea045820f0909affaa7ee7abe5dd4e100598d4dc53cd709d5a5c2cac40e7412f232f7c9c06d90130a2018200f4021abd16bee507d90130a1018600f400f480f4").unwrap();
    let data: Value = serde_cbor::from_slice(&inp).unwrap();
    let mut out = String::new();
    parse_ur_desc(data, &mut out);
    println!("\noutput descriptor: {:?}", out);

    // TODO this test case is incorrect in the spec, because it is missing
    // the parent fingeprint in cbor notation

    Ok(())
}

#[test]
fn outputdesc_test_vector_4() -> Result<(), Box<dyn std::error::Error>> {
    let  inp = hex::decode("D90193D9012FA503582102D2B36900396C9282FA14628566582F206A5DD0BCC8D5E892611806CAFB0301F0045820637807030D55D01F9A0CB3A7839515D796BD07706386A6EDDF06CC29A65A0E2906D90130A20186182CF500F500F5021AD34DB33F07D90130A1018401F480F4081A78412E3A").unwrap();
    let _expected = "wsh(multi(1,xpub661MyMwAqRbcFW31YEwpkMuc5THy2PSt5bDMsktWQcFF8syAmRUapSCGu8ED9W6oDMSgv6Zz8idoc4a6mr8BDzTJY47LJhkJ8UB7WEGuduB/1/0/*,[m/0]xpub67tVq9TC3jGc8hyd7kgmC1GK87PYAtgqFcAhJTgBP5VQ6d9RssQK1iwWk3ZY8cbrAuwmp31gShjmBoHKmKbEaQfAbppVSuDh1ojtymY92dh/0/0/*))";

    let data: Value = serde_cbor::from_slice(&inp).unwrap();
    let mut out = String::new();
    parse_ur_desc(data, &mut out);
    println!("\noutput descriptor: {:?}", out);

    // TODO this test case is incorrect in the spec, it contains incorrect depth

    Ok(())
}

#[test]
fn outputdesc_test_vector_3() -> Result<(), Box<dyn std::error::Error>> {
    let inp = hex::decode("d90190d90196a201020282d90132a1035821022f01e5e15cca351daff3843fb70f3c2f0a1bdd05e5af888a67784ef3e10a2a01d90132a103582103acd484e2f0c7f65309ad178a9f559abde09796974c57e714c35f110dfc27ccbe").unwrap();
    let expected = "sh(multi(2,022f01e5e15cca351daff3843fb70f3c2f0a1bdd05e5af888a67784ef3e10a2a01,03acd484e2f0c7f65309ad178a9f559abde09796974c57e714c35f110dfc27ccbe))";
    let data: Value = serde_cbor::from_slice(&inp).unwrap();
    let mut out = String::new();
    parse_ur_desc(data, &mut out);

    // This test vector is correct
    assert_eq!(out, expected);

    Ok(())
}

#[test]
fn hdkey_test_vector_1() -> Result<(), Box<dyn std::error::Error>> {
    let mut inp =
        hex::decode("A301F503582100E8F32E723DECF4051AEFAC8E2C93C9C5B214313817CDB01A1494B917C8436B35045820873DFF81C02F525623FD1FE5167EAC3A55A049DE3D314BB42EE227FFED37D508").unwrap();
    let key_data_expected =
        hex::decode("00e8f32e723decf4051aefac8e2c93c9c5b214313817cdb01a1494b917c8436b35").unwrap();
    let chaincode_expected =
        hex::decode("873dff81c02f525623fd1fe5167eac3a55a049de3d314bb42ee227ffed37d508").unwrap();

    let hdkey: HDKey = serde_cbor::de::from_mut_slice(&mut inp[..]).unwrap();
    println!("hdkey: {:?}", hdkey);

    assert_eq!(hdkey.is_master.unwrap(), true);
    assert_eq!(hdkey.key_data, key_data_expected);
    assert_eq!(hdkey.chain_code.unwrap(), chaincode_expected);
    assert_eq!(hdkey.origin, None);
    assert_eq!(hdkey.use_info, None);

    Ok(())
}

#[test]
fn psbt_test_vector_1() -> Result<(), Box<dyn std::error::Error>> {
    let inp = hex::decode("70736274FF01009A020000000258E87A21B56DAF0C23BE8E7070456C336F7CBAA5C8757924F545887BB2ABDD750000000000FFFFFFFF838D0427D0EC650A68AA46BB0B098AEA4422C071B2CA78352A077959D07CEA1D0100000000FFFFFFFF0270AAF00800000000160014D85C2B71D0060B09C9886AEB815E50991DDA124D00E1F5050000000016001400AEA9A2E5F0F876A588DF5546E8742D1D87008F000000000000000000").unwrap();
    let expected = "ur:crypto-psbt/hdosjojkidjyzmadaenyaoaeaeaeaohdvsknclrejnpebncnrnmnjojofejzeojlkerdonspkpkkdkykfelokgprpyutkpaeaeaeaeaezmzmzmzmlslgaaditiwpihbkispkfgrkbdaslewdfycprtjsprsgksecdratkkhktikewdcaadaeaeaeaezmzmzmzmaojopkwtayaeaeaeaecmaebbtphhdnjstiambdassoloimwmlyhygdnlcatnbggtaevyykahaeaeaeaecmaebbaeplptoevwwtyakoonlourgofgvsjydpcaltaemyaeaeaeaeaeaeaeaeaebkgdcarh";

    assert_eq!(psbt_as_ur(inp), expected);

    Ok(())
}

#[test]
fn address_test_vector_1() -> Result<(), Box<dyn std::error::Error>> {
    let inp = "1BvBMSEYstWetqTFn5Au4m4GFg7xJaNVN2".to_string();
    let ad = bdk::bitcoin::Address::from_str(&inp).unwrap();

    let pubkeyhash = "77bff20c60e522dfaa3350c39b030a5d004e839a";
    let pubkeyhash = hex::decode(pubkeyhash).unwrap();

    let addr = bdk::bitcoin::Address {
        payload: bdk::bitcoin::util::address::Payload::PubkeyHash(
            bdk::bitcoin::hash_types::PubkeyHash::from_slice(&pubkeyhash).unwrap(),
        ),
        network: bdk::bitcoin::Network::Bitcoin,
    };

    println!("**addr {:?}", addr);

    assert_eq!(addr, ad);

    Ok(())
}
