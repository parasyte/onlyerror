use myn::prelude::*;
use proc_macro::{Delimiter, Ident, TokenStream};
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Debug)]
pub(crate) struct Error {
    pub(crate) name: Ident,
    pub(crate) variants: Vec<Variant>,
}

#[derive(Debug)]
pub(crate) struct Variant {
    pub(crate) name: Ident,
    pub(crate) ty: VariantType,
    pub(crate) fields: HashMap<Rc<str>, String>,
    pub(crate) display: String,
    pub(crate) display_fields: Vec<Rc<str>>,
    pub(crate) source: ErrorSource,
}

#[derive(Debug, PartialEq)]
pub(crate) enum VariantType {
    Unit,
    Tuple,
    Struct,
}

#[derive(Debug)]
pub(crate) struct Field {
    attrs: Vec<Attribute>,
    path: String,
}

#[derive(Debug)]
pub(crate) enum ErrorSource {
    None,
    From(Rc<str>),
    Source(Rc<str>),
}

#[derive(Debug)]
pub(crate) struct OrderedMap<T> {
    keys: Vec<Rc<str>>,
    map: HashMap<Rc<str>, T>,
}

impl Error {
    pub(crate) fn parse(input: TokenStream) -> Result<Self, TokenStream> {
        let mut input = input.into_token_iter();
        input.parse_attributes()?;
        input.parse_visibility()?;
        input.expect_ident("enum")?;
        let name = input.as_ident()?;

        let mut content = input.expect_group(Delimiter::Brace)?;
        let mut variants = vec![];

        while content.peek().is_some() {
            variants.push(Variant::parse(&mut content)?);
        }

        match input.next() {
            None => Ok(Self { name, variants }),
            tree => Err(spanned_error("Unexpected token", tree.as_span())),
        }
    }
}

impl Variant {
    pub(crate) fn parse(input: &mut TokenIter) -> Result<Self, TokenStream> {
        let attrs = input.parse_attributes()?;
        let name = input.as_ident()?;

        let mut fields = HashMap::new();
        let mut source = ErrorSource::None;
        let ty = if let Ok(group) = input.as_group() {
            let (ty, map) = match group.delimiter() {
                Delimiter::Parenthesis => (VariantType::Tuple, parse_tuple_fields(group.stream())?),
                Delimiter::Brace => (VariantType::Struct, parse_struct_fields(group.stream())?),
                _ => return Err(spanned_error("Unexpected delimiter", group.span())),
            };

            // Resolve error source.
            let num_fields = map.len();
            for (key, field) in map.into_iter() {
                let attrs = field
                    .attrs
                    .iter()
                    .filter(|attr| ["from", "source"].contains(&attr.name.to_string().as_str()));

                for attr in attrs {
                    // De-dupe.
                    if let Some(name) = source.as_ref() {
                        let msg = format!(
                            "#[from] | #[source] can only be used once. \
                            Previously seen on field `{name}`"
                        );

                        return Err(spanned_error(msg, attr.name.span()));
                    }

                    if attr.name.to_string() == "from" {
                        if num_fields > 1 {
                            return Err(spanned_error(
                                "#[from] can only be used with a single field",
                                name.span(),
                            ));
                        }

                        source = ErrorSource::From(key.clone());
                    } else {
                        source = ErrorSource::Source(key.clone());
                    }
                }

                fields.insert(key, field.path);
            }

            let _ = input.expect_punct(',');

            ty
        } else {
            VariantType::Unit
        };

        // #[error] attributes override doc comments
        let display = if let Some(mut tree) = attrs
            .iter()
            .find_map(|attr| (attr.name.to_string() == "error").then_some(attr.tree.clone()))
            .and_then(|mut tree| tree.expect_group(Delimiter::Parenthesis).ok())
        {
            let mut string = tree.as_lit()?.as_string()?;

            if ty == VariantType::Tuple {
                // Replace field references
                for i in 0..fields.len() {
                    string = string
                        .replace(&format!("{{{i}:"), &format!("{{field_{i}:"))
                        .replace(&format!("{{{i}}}"), &format!("{{field_{i}}}"));
                }
            }

            string
        } else {
            get_doc_comment(&attrs).join("")
        };
        let display_fields = display
            .split('{')
            .skip(1)
            .filter_map(|s| s.split('}').next())
            .filter_map(|s| s.split(':').next())
            .map(Rc::from)
            .collect();

        Ok(Self {
            name,
            ty,
            fields,
            display,
            display_fields,
            source,
        })
    }
}

fn parse_tuple_fields(input: TokenStream) -> Result<OrderedMap<Field>, TokenStream> {
    let mut input = input.into_token_iter();
    let mut fields = OrderedMap::new();
    let mut index = 0;

    while input.peek().is_some() {
        let field = parse_tuple_field(&mut input)?;
        fields.insert(index.to_string().into(), field);
        index += 1;
    }

    Ok(fields)
}

fn parse_tuple_field(input: &mut TokenIter) -> Result<Field, TokenStream> {
    let attrs = input.parse_attributes()?;
    let (path, _) = input.parse_path()?;
    let _ = input.expect_punct(',');

    Ok(Field { attrs, path })
}

fn parse_struct_fields(input: TokenStream) -> Result<OrderedMap<Field>, TokenStream> {
    let mut input = input.into_token_iter();
    let mut fields = OrderedMap::new();

    while input.peek().is_some() {
        let (name, field) = parse_struct_field(&mut input)?;
        fields.insert(name.into(), field);
    }

    Ok(fields)
}

fn parse_struct_field(input: &mut TokenIter) -> Result<(String, Field), TokenStream> {
    let attrs = input.parse_attributes()?;
    let name = input.as_ident()?;
    input.expect_punct(':')?;
    let (path, _) = input.parse_path()?;
    let _ = input.expect_punct(',');

    Ok((name.to_string(), Field { attrs, path }))
}

impl ErrorSource {
    fn as_ref(&self) -> Option<&Rc<str>> {
        match self {
            Self::None => None,
            Self::From(name) | Self::Source(name) => Some(name),
        }
    }
}

impl<T> OrderedMap<T> {
    fn new() -> Self {
        Self {
            keys: vec![],
            map: HashMap::new(),
        }
    }

    fn len(&self) -> usize {
        self.keys.len()
    }

    fn into_iter(mut self) -> impl Iterator<Item = (Rc<str>, T)> {
        self.keys.into_iter().map(move |key| {
            let value = self.map.remove(&key).unwrap();

            (key, value)
        })
    }

    fn insert(&mut self, key: Rc<str>, value: T) -> Option<T> {
        let result = self.map.insert(key.clone(), value);

        if result.is_none() {
            self.keys.push(key);
        }

        result
    }
}
