#![allow(dead_code)]

#[derive(Debug, onlyerror::Error)]
enum Error {
    /// First
    First,
    #[error("Second with {0}")]
    Second(usize),
    #[error("Third with {key} and {value:?}")]
    Third { key: String, value: Vec<usize> },
}

fn main() {}
