//! `no_std`-compatible derive macro for [`Error`].

#![forbid(unsafe_code)]
#![deny(clippy::all)]
#![deny(clippy::pedantic)]

use crate::parser::{Error, ErrorSource, VariantType};
use myn::utils::spanned_error;
use proc_macro::{Span, TokenStream};
use std::str::FromStr as _;

mod parser;

#[proc_macro_derive(Error, attributes(error, from, source))]
pub fn derive_error(input: TokenStream) -> TokenStream {
    let ast = match Error::parse(input) {
        Ok(ast) => ast,
        Err(err) => return err,
    };

    let name = &ast.name;
    let error_impl = ast
        .variants
        .iter()
        .filter_map(|v| match &v.source {
            ErrorSource::From(index) | ErrorSource::Source(index) => {
                let name = &v.name;

                if v.ty == VariantType::Tuple {
                    // TODO: Support more than one field for #[source]
                    Some(format!("Self::{name}(field) => Some(field),"))
                } else {
                    Some(format!("Self::{name} {{ {index}, ..}} => Some({index}),"))
                }
            }
            ErrorSource::None => None,
        })
        .collect::<String>();
    let display_impl = ast
        .variants
        .iter()
        .map(|v| {
            let name = &v.name;
            let display = &v.display;
            let fields = v
                .display_fields
                .iter()
                .map(|field| format!("{field},"))
                .collect::<String>();

            if v.ty == VariantType::Tuple {
                // TODO: Support more than one field for #[source]
                format!(r#"Self::{name}(_) => write!(f, {display:?})?,"#)
            } else {
                format!(r#"Self::{name} {{ {fields} .. }} => write!(f, {display:?})?,"#)
            }
        })
        .collect::<String>();
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
                    r#"impl ::std::convert::From<{from_ty}> for {name} {{
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
            impl ::std::error::Error for {name} {{
                fn source(&self) -> Option<&(dyn ::std::error::Error + 'static)> {{
                    match self {{
                        {error_impl}
                        _ => None,
                    }}
                }}
            }}

            impl ::std::fmt::Display for {name} {{
                fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> Result<(), ::std::fmt::Error> {{
                    match self {{
                        {display_impl}
                        _ => (),
                    }}
                    Ok(())
                }}
            }}

            {from_impls}
        "#
    ));

    match code {
        Ok(stream) => stream,
        Err(err) => spanned_error(err.to_string(), Span::call_site()),
    }
}
