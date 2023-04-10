use error_iter::ErrorIter as _;
use onlyerror::Error;
use std::process::ExitCode;

/// All of my errors.
#[derive(Debug, Error)]
enum Error {
    /// I/O error with context.
    #[error("I/O error: {ctx}.")]
    IoContext {
        /// The error source.
        #[source]
        source: std::io::Error,

        /// Additional context.
        ctx: String,
    },

    /// Parse error.
    Parse(#[from] std::num::ParseFloatError),
}

fn run() -> Result<(), Error> {
    let path = "/foo/bar/does-not-exist";
    let contents = std::fs::read_to_string(path).map_err(|source| Error::IoContext {
        source,
        ctx: format!("While opening path `{path}`"),
    })?;

    for line in contents.lines() {
        let num: f32 = line.parse()?;
        println!("Num: {num}");
    }

    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(_) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("Error: {err}");
            for source in err.sources().skip(1) {
                eprintln!("  Caused by: {source}");
            }

            ExitCode::FAILURE
        }
    }
}
