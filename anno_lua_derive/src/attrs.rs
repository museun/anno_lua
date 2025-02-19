use std::collections::{BTreeMap, HashMap};

use proc_macro2::Span;
use syn::{spanned::Spanned, Attribute, LitStr};

#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub enum Kind {
    Type,
    Name,
    Ignore,
}

#[derive(Debug)]
pub struct Attr {
    pub key: Span,
    pub value: Span,
    pub data: String,
}

pub fn parse_attrs(
    attrs: &[Attribute],
    allowed: &[(&'static str, Kind)],
) -> Result<HashMap<Kind, Attr>, syn::Error> {
    let map: BTreeMap<&'static str, Kind> = allowed.iter().copied().collect();

    let Some(attr) = attrs.iter().find(|c| c.path().is_ident("anno")) else {
        return Ok(HashMap::new());
    };

    let mut errors = vec![];
    let mut out = HashMap::new();

    attr.meta.require_list()?.parse_nested_meta(|meta| {
        let path = &meta.path;

        if let Some(id) = path.get_ident() {
            if map.get(&*id.to_string()) == Some(&Kind::Ignore) {
                let attr = Attr {
                    key: meta.path.span(),
                    value: meta.path.span(),
                    data: String::new(),
                };
                out.insert(Kind::Ignore, attr);
                return Ok(());
            }
        }

        let ident = path.require_ident()?;
        let raw = ident.to_string();

        let kind = map.get(&*raw).ok_or_else(|| {
            let available = map.keys().fold(String::new(), |mut a, c| {
                if !a.is_empty() {
                    a.push_str(", ");
                }
                a.push_str(c);
                a
            });

            syn::Error::new(
                path.span(),
                format!("unknown ident: {raw}, supported: {available}",),
            )
        });

        let kind = match kind {
            Ok(kind) => *kind,
            Err(err) => {
                let _ = meta.value()?.parse::<LitStr>()?;
                errors.push(err);
                return Ok(());
            }
        };

        let value = meta.value()?;
        let value_span = value.span();
        let value = value.parse::<LitStr>()?.value();

        if value.trim().is_empty() {
            errors.push(syn::Error::new(value_span, "attribute cannot be empty"));
            return Ok(());
        }

        if let Some(Attr { key: previous, .. }) = out.insert(
            kind,
            Attr {
                key: meta.path.span(),
                value: value_span,
                data: value,
            },
        ) {
            let mut err = syn::Error::new(path.span(), "duplicate attribute found");
            err.combine(syn::Error::new(previous, "previous use here"));
            errors.push(err);
        }
        Ok(())
    })?;

    errors.reverse();

    if let Some(combined) = errors.into_iter().reduce(|mut left, right| {
        left.combine(right);
        left
    }) {
        return Err(combined);
    }

    Ok(out)
}
