#![allow(dead_code)]

#[derive(Debug, onlyerror::Error)]
enum Error {
    #[error("One")]
    One,
}

fn main() {}
