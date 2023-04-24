#[derive(Debug, onlyerror::Error)]
enum Error {
    First,
    Second(usize),
    Third { key: String, value: Vec<usize> },
}

fn main() {}
