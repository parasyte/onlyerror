#![allow(dead_code)]

#[derive(Debug, onlyerror::Error)]
#[no_display]
enum Error {
    First,
    Second(usize),
    Third { key: String, value: Vec<usize> },
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::result::Result<(), core::fmt::Error> {
        write!(f, "Should work")
    }
}

fn main() {}
