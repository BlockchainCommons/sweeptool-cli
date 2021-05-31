//extern crate bitcoin;
extern crate hex;
extern crate serde_json;

#[derive(Debug)]
pub struct SweepError {
    kind: String,
    message: String,
}

impl From<serde_json::Error> for SweepError {
    fn from(error: serde_json::Error) -> Self {
        SweepError {
            kind: String::from("serde_json"),
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
