use std::collections::{HashMap, VecDeque};

use quote::quote;
use syn::{spanned::Spanned, DataStruct, DeriveInput, Fields, LitStr};

use crate::{
    attrs::{parse_attrs, Attr, Kind},
    data,
    docs::collect_docs,
    error::Error,
};

struct ClassMeta {
    exact: bool,
    guess: bool,
    name: String,
}

impl ClassMeta {
    fn parse(input: &DeriveInput) -> Result<Self, Error> {
        let Some(attr) = input.attrs.iter().find(|c| c.path().is_ident("anno")) else {
            return Ok(Self {
                exact: false,
                guess: false,
                name: input.ident.to_string(),
            });
        };

        let mut this = Self {
            exact: false,
            guess: false,
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

            if meta.path.is_ident("guess") {
                this.guess = true;
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

    let fields = match collect_fields(&data.fields, meta.guess) {
        Ok(fields) => fields,
        Err(err) => return err.into_compile_error(),
    };

    let ClassMeta { exact, name, .. } = meta;

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

fn collect_fields(fields: &Fields, guess: bool) -> Result<Vec<data::Field>, Error> {
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

                let ty = kvs.remove(&Kind::Type).map(|Attr { data, .. }| data);
                let ty = if guess {
                    ty.unwrap_or_else(|| {
                        if let syn::Type::Path(path) = &field.ty {
                            try_classify_type(&path.path)
                        } else {
                            None
                        }
                        .unwrap_or_else(|| "any".to_string())
                    })
                } else {
                    ty.ok_or_else(|| Error::TyRequire(field.ident.span()))?
                };

                let new = data::Field {
                    name,
                    ty,
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

#[derive(Copy, Clone, Debug, Default, PartialEq)]
enum Container {
    #[default]
    None,
    Option,
    Vec,
}

fn try_classify_type(path: &syn::Path) -> Option<String> {
    let mut queue = VecDeque::from_iter([(String::new(), path)]);

    while let Some((mut buf, path)) = queue.pop_front() {
        let ident = match path.get_ident() {
            Some(ident) => ident,
            None => {
                if path.segments.len() > 1 {
                    return None;
                }
                let head = path.segments.first()?;

                let container = match () {
                    _ if head.ident == "Option" => Container::Option,
                    _ if head.ident == "Vec" => Container::Vec,
                    _ => return None,
                };

                let syn::PathArguments::AngleBracketed(syn::AngleBracketedGenericArguments {
                    args,
                    ..
                }) = &head.arguments
                else {
                    return None;
                };

                if args.len() > 1 {
                    return None;
                }

                let syn::GenericArgument::Type(syn::Type::Path(path)) = args.first()? else {
                    return None;
                };

                match container {
                    Container::None => {}
                    Container::Option => buf.push_str("?"),
                    Container::Vec => buf.push_str("[]"),
                }

                queue.push_back((buf, &path.path));
                continue;
            }
        };

        if ident == "String" {
            return Some(format!("string{buf}"));
        }

        if ident == "f32" || ident == "f64" {
            return Some(format!("number{buf}"));
        }

        if ident == "bool" {
            return Some(format!("boolean{buf}"));
        }

        if [
            "i8", "i16", "i32", "i64", "isize", //
            "u8", "u16", "u32", "u64", "usize",
        ]
        .iter()
        .any(|c| ident == c)
        {
            return Some(format!("integer{buf}"));
        }
    }

    None
}
