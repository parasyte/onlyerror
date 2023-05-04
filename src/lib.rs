//! `no_std`-compatible derive macro for error handling.
//!
//! `onlyerror` is comparable in feature set to the venerable [`thiserror`] crate with two major
//! differences:
//!
//! 1. The feature subset is highly restricted.
//! 2. Generally much faster compile times.
//!
//! For more on compile times, see the [`myn` benchmarks].
//!
//! # Example
//!
//! ```
//! use onlyerror::Error;
//!
//! #[derive(Debug, Error)]
//! pub enum HttpClientError {
//!     /// I/O error.
//!     Io(#[from] std::io::Error),
//!
//!     /// Login error.
//!     #[error("Login error. Server message: `{0}`.")]
//!     LoginError(String),
//!
//!     /// Invalid header.
//!     #[error("Invalid header (expected {expected:?}, found {found:?}).")]
//!     InvalidHeader {
//!         expected: String,
//!         found: String,
//!     },
//!
//!     /// Unknown.
//!     Unknown,
//! }
//! ```
//!
//! # DSL reference
//!
//! The macro has a DSL modeled after `thiserror`, so it should feel familiar to anyone who has used
//! it.
//!
//! - The macro derives an implementation for the `Error` trait.
//! - `Display` is derived using the `#[error("...")]` attributes with a fallback to doc comments.
//! - `From` is derived for each `#[from]` or `#[source]` attribute.
//!
//! Error messages in `#[error("...")]` can reference enum variant fields by name (for struct-like
//! variants) or by number (for tuple-like variants) using the [`std::fmt`] machinery.
//!
//! It is recommended to use `#[error("...")]` when you need interpolation, otherwise use doc
//! comments. Doing this will keep implementation details out of your documentation while making
//! the error variants self-documenting.
//!
//! # Limitations
//!
//! - Only `enum` types are supported by the [`Error`] macro.
//! - Only inline string interpolations are supported by the derived `Display` impl.
//! - Either all variants must be given an error message, or `#[no_display]` attribute must be set
//!   to enum with hand-written `Display` implementation
//! - `From` impls are only derived for `#[from]` and `#[source]` attributes, not implicitly for any
//!   field names.
//! - `Backtrace` is not supported.
//! - `#[error(transparent)]` is not supported.
//!
//! # Cargo features
//!
//! - `std` (default): use the [`std::error`] module.
//!
//! To use `onlyerror` in a `no_std` environment, disable default features in your Cargo manifest.
//!
//! As of writing, you must add `#![feature(error_in_core)]` to the top-level `lib.rs` or `main.rs`
//! file to enable the [`core::error`] module. This feature flag is only available on nightly
//! compilers.
//!
//! [`Error`]: derive@Error
//! [`myn` benchmarks]: https://github.com/parasyte/myn/blob/main/benchmarks.md
//! [`thiserror`]: https://docs.rs/thiserror

#![forbid(unsafe_code)]
#![deny(clippy::all)]
#![deny(clippy::pedantic)]
#![allow(clippy::let_underscore_untyped)]

use crate::parser::{Error, ErrorSource, VariantType};
use myn::utils::spanned_error;
use proc_macro::{Span, TokenStream};
use std::{rc::Rc, str::FromStr as _};

mod parser;

#[allow(clippy::too_many_lines)]
#[proc_macro_derive(Error, attributes(error, from, source, no_display))]
pub fn derive_error(input: TokenStream) -> TokenStream {
    let ast = match Error::parse(input) {
        Ok(ast) => ast,
        Err(err) => return err,
    };

    #[cfg(feature = "std")]
    let std_crate = "std";
    #[cfg(not(feature = "std"))]
    let std_crate = "core";

    let name = &ast.name;
    let error_matches = ast
        .variants
        .iter()
        .filter_map(|v| match &v.source {
            ErrorSource::From(index) | ErrorSource::Source(index) => {
                let name = &v.name;

                Some(match &v.ty {
                    VariantType::Unit => format!("Self::{name} => None,"),
                    VariantType::Tuple => {
                        let index_num: usize = index.parse().unwrap_or_default();
                        let fields = (0..v.fields.len())
                            .map(|i| if i == index_num { "field," } else { "_," })
                            .collect::<String>();

                        format!("Self::{name}({fields}) => Some(field),")
                    }
                    VariantType::Struct => {
                        format!("Self::{name} {{ {index}, ..}} => Some({index}),")
                    }
                })
            }
            ErrorSource::None => None,
        })
        .collect::<String>();

    let display_impl = if ast.no_display {
        String::new()
    } else {
        let display = ast.variants.iter().map(|v| {
            let name = &v.name;
            let display = &v.display;

            if display.is_empty() {
                return Err(name);
            }

            let display_fields = v
                .display_fields
                .iter()
                .map(|field| format!("{field},"))
                .collect::<String>();

            Ok(match &v.ty {
                VariantType::Unit => format!("Self::{name} => write!(f, {display:?}),"),
                VariantType::Tuple => {
                    let fields = (0..v.fields.len())
                        .map(|i| {
                            if v.display_fields.contains(&Rc::from(format!("field_{i}"))) {
                                format!("field_{i},")
                            } else {
                                "_,".to_string()
                            }
                        })
                        .collect::<String>();
                    format!("Self::{name}({fields}) => write!(f, {display:?}, {display_fields}),")
                }
                VariantType::Struct => {
                    format!(
                        "Self::{name} {{ {display_fields} .. }} => \
                        write!(f, {display:?}, {display_fields}),"
                    )
                }
            })
        });
        let mut display_matches = String::new();
        for res in display {
            match res {
                Err(name) => {
                    return spanned_error("Required error message is missing", name.span());
                }
                Ok(msg) => display_matches.push_str(&msg),
            }
        }
        let display_matches = if display_matches.is_empty() {
            String::from("Ok(())")
        } else {
            format!("match self {{ {display_matches} }}")
        };

        format!(
            r#"impl ::{std_crate}::fmt::Display for {name} {{
                fn fmt(&self, f: &mut ::{std_crate}::fmt::Formatter<'_>) ->
                    ::{std_crate}::result::Result<(), ::{std_crate}::fmt::Error>
                {{
                    {display_matches}
                }}
            }}"#
        )
    };

    let from_impls = ast
        .variants
        .into_iter()
        .filter_map(|v| match v.source {
            ErrorSource::From(index) => {
                let variant_name = v.name;
                let from_ty = &v.fields[&index];
                let body = if v.ty == VariantType::Tuple {
                    format!(r#"Self::{variant_name}(value)"#)
                } else {
                    format!(r#"Self::{variant_name} {{ {index}: value }}"#)
                };

                Some(format!(
                    r#"impl ::{std_crate}::convert::From<{from_ty}> for {name} {{
                        fn from(value: {from_ty}) -> Self {{
                            {body}
                        }}
                    }}"#
                ))
            }
            _ => None,
        })
        .collect::<String>();

    let code = TokenStream::from_str(&format!(
        r#"
            impl ::{std_crate}::error::Error for {name} {{
                fn source(&self) -> Option<&(dyn ::{std_crate}::error::Error + 'static)> {{
                    match self {{
                        {error_matches}
                        _ => None,
                    }}
                }}
            }}

            {display_impl}
            {from_impls}
        "#
    ));

    match code {
        Ok(stream) => stream,
        Err(err) => spanned_error(err.to_string(), Span::call_site()),
    }
}
