#![allow(dead_code)]

#[derive(Debug, onlyerror::Error)]
enum Error {
    /// Skip positional tuple fields {1}
    Skip(u8, u8),
}

fn main() {}
