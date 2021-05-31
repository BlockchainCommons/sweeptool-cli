//extern crate bitcoin;
extern crate hex;
extern crate serde_json;
use ur_rs::bytewords;

#[derive(Debug)]
pub struct SweepError {
    kind: String,
    message: String,
}

impl SweepError {
    pub fn new(kind: String, message: String) -> Self {
        SweepError { kind, message }
    }
}

impl From<bdk::bitcoin::util::key::Error> for SweepError {
    fn from(error: bdk::bitcoin::util::key::Error) -> Self {
        SweepError {
            kind: String::from("Address"),
            message: error.to_string(),
        }
    }
}

use bdk::bitcoin::util::address::Error as AddressError;
impl From<AddressError> for SweepError {
    fn from(error: AddressError) -> Self {
        SweepError {
            kind: String::from("Address"),
            message: error.to_string(),
        }
    }
}

impl From<serde_json::Error> for SweepError {
    fn from(error: serde_json::Error) -> Self {
        SweepError {
            kind: String::from("serde_json"),
            message: error.to_string(),
        }
    }
}

impl From<bytewords::Error> for SweepError {
    fn from(error: bytewords::Error) -> Self {
        SweepError {
            kind: String::from("cbor"),
            message: error.to_string(),
        }
    }
}

impl From<bdk::Error> for SweepError {
    fn from(error: bdk::Error) -> Self {
        SweepError {
            kind: String::from("bdk"),
            message: error.to_string(),
        }
    }
}

impl From<bdk::electrum_client::Error> for SweepError {
    fn from(error: bdk::electrum_client::Error) -> Self {
        SweepError {
            kind: String::from("electrum client"),
            message: error.to_string(),
        }
    }
}

impl From<serde_cbor::Error> for SweepError {
    fn from(error: serde_cbor::Error) -> Self {
        SweepError {
            kind: String::from("serde_cbor"),
            message: error.to_string(),
        }
    }
}
