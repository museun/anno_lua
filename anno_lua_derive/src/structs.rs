use std::collections::HashMap;

use quote::quote;
use syn::{spanned::Spanned, DataStruct, DeriveInput, Fields, LitStr};

use crate::{
    attrs::{parse_attrs, Attr, Kind},
    data,
    docs::collect_docs,
    error::Error,
};

pub struct ClassMeta {
    pub exact: bool,
    pub name: String,
}

impl ClassMeta {
    pub fn parse(input: &DeriveInput) -> Result<Self, Error> {
        let Some(attr) = input.attrs.iter().find(|c| c.path().is_ident("anno")) else {
            return Ok(Self {
                exact: false,
                name: input.ident.to_string(),
            });
        };

        let mut this = Self {
            exact: false,
            name: String::new(),
        };

        attr.meta.require_list()?.parse_nested_meta(|meta| {
            if meta.path.is_ident("name") {
                if !this.name.is_empty() {
                    return Err(Error::DuplicateName(meta.path.span()).into_syn_error());
                }
                let value = meta.value()?;
                let name = value.parse::<LitStr>()?.value();
                if name.trim().is_empty() {
                    return Err(Error::EmptyName(value.span()).into_syn_error());
                }
                this.name = name;
            }

            if meta.path.is_ident("exact") {
                this.exact = true;
            }

            Ok(())
        })?;

        if this.name.trim().is_empty() {
            this.name = input.ident.to_string()
        }

        Ok(this)
    }
}

pub fn parse(input: &DeriveInput, data: &DataStruct) -> proc_macro::TokenStream {
    let docs = collect_docs(&input.attrs);
    let meta = match ClassMeta::parse(input) {
        Ok(meta) => meta,
        Err(err) => return err.into_compile_error(),
    };

    let fields = match collect_fields(&data.fields) {
        Ok(fields) => fields,
        Err(err) => return err.into_compile_error(),
    };

    let ClassMeta { exact, name } = meta;

    let iter = fields.iter().map(|data::Field { name, ty, docs }| {
        quote! {
            anno_lua::Field {
                name: #name,
                ty: #ty,
                docs: &[ #( #docs ),* ]
            }
        }
    });

    let ident = &input.ident;
    let ast = quote! {
        impl anno_lua::Anno for #ident {
            fn lua_type() -> anno_lua::Type {
                anno_lua::Type::Class(anno_lua::Class{
                    exact: #exact,
                    docs: &[ #( #docs ),* ],
                    name: #name,
                    fields: &[ #( #iter ),* ],
                })
            }
        }
    };

    ast.into()
}

fn collect_fields(fields: &Fields) -> Result<Vec<data::Field>, Error> {
    let mut out = vec![];
    let mut errors = vec![];

    let mut seen = HashMap::new();

    for field in fields {
        let mut kvs = match parse_attrs(
            &field.attrs,
            &[
                ("lua_type", Kind::Type),
                ("name", Kind::Name),
                ("ignore", Kind::Ignore),
            ],
        ) {
            Ok(kvs) => kvs,
            Err(err) => {
                errors.push(err);
                continue;
            }
        };

        match &field.ident {
            Some(name) => {
                if kvs.remove(&Kind::Ignore).is_some() {
                    continue;
                }

                let Attr {
                    value, data: name, ..
                } = kvs.remove(&Kind::Name).unwrap_or_else(|| Attr {
                    key: field.ident.span(),
                    value: field.ident.span(),
                    data: name.to_string(),
                });

                // if we don't have an attribute, use the field name
                let new = data::Field {
                    name,

                    ty: kvs
                        .remove(&Kind::Type)
                        .map(|Attr { data, .. }| data)
                        .ok_or_else(|| Error::TyRequire(field.ident.span()))?,

                    docs: collect_docs(&field.attrs),
                };

                if let Some(prev) = seen.insert(new.name.clone(), value) {
                    let mut err = syn::Error::new(value, "duplicate name found");
                    err.combine(syn::Error::new(prev, "previous used here"));
                    errors.push(err);
                    continue;
                }

                out.push(new)
            }
            None => return Err(Error::UnnamedField(field.span())),
        }
    }

    errors.reverse();

    if let Some(combined) = errors.into_iter().reduce(|mut left, right| {
        left.combine(right);
        left
    }) {
        return Err(combined.into());
    }

    Ok(out)
}
