use std::collections::HashMap;

use quote::quote;
use syn::{
    spanned::Spanned, DataEnum, DeriveInput, Expr, ExprLit, ExprUnary, Fields, Lit, LitStr, UnOp,
    Variant,
};

use crate::{
    attrs::{parse_attrs, Attr, Kind},
    data,
    docs::collect_docs,
    error::Error,
};

pub struct EnumMeta {
    pub use_self: bool,
    pub name: String,
}

impl EnumMeta {
    pub fn parse(input: &DeriveInput) -> Result<Self, syn::Error> {
        let Some(attr) = input.attrs.iter().find(|c| c.path().is_ident("anno")) else {
            return Ok(Self {
                use_self: false,
                name: input.ident.to_string(),
            });
        };

        let mut this = Self {
            use_self: false,
            name: String::new(),
        };

        attr.meta.require_list()?.parse_nested_meta(|meta| {
            if meta.path.is_ident("name") {
                if !this.name.is_empty() {
                    return Err(syn::Error::new(meta.path.span(), "duplicate name provided"));
                }
                let value = meta.value()?;
                let name = value.parse::<LitStr>()?.value();
                if name.trim().is_empty() {
                    return Err(syn::Error::new(value.span(), "name cannot be empty"));
                }
                this.name = name;
            }

            if meta.path.is_ident("self") {
                this.use_self = true;
            }

            Ok(())
        })?;

        if this.name.trim().is_empty() {
            this.name = input.ident.to_string()
        }

        Ok(this)
    }
}

pub fn parse(input: &DeriveInput, data: &DataEnum) -> proc_macro::TokenStream {
    let docs = collect_docs(&input.attrs);
    let meta = match EnumMeta::parse(input) {
        Ok(meta) => meta,
        Err(err) => return err.into_compile_error().into(),
    };

    let variants = data.variants.iter().collect::<Vec<_>>();
    let variants = match collect_variants(&variants, &meta.name, meta.use_self) {
        Ok(variants) => variants,
        Err(err) => return err.into_compile_error(),
    };

    let anno_enum = make_variant_mapping(&input.ident, &variants);

    let EnumMeta { name, .. } = meta;
    let iter = variants.iter().map(
        |data::Variant {
             name: lua_name,
             discriminant,
             docs,
             ..
         }| {
            let discriminant = match discriminant {
                data::Discriminant::Named(n) => {
                    quote! {
                        anno_lua::Discriminant::Named(#n)
                    }
                }
                data::Discriminant::Number(n) => {
                    quote! {
                        anno_lua::Discriminant::Number(#n)
                    }
                }
            };

            quote! {
                anno_lua::Variant {
                    name: #lua_name,
                    discriminant: #discriminant,
                    docs: &[ #( #docs ),* ]
                }
            }
        },
    );

    let ident = &input.ident;
    let ast = quote! {
        impl anno_lua::Anno for #ident {
            fn lua_type() -> anno_lua::Type {
                anno_lua::Type::Enum(anno_lua::Enum {
                    docs: &[ #( #docs ),* ],
                    name: #name,
                    variants: &[ #( #iter ),* ],
                })
            }
        }

        #anno_enum
    };

    ast.into()
}

fn make_variant_mapping(
    ident: &syn::Ident,
    variants: &[data::Variant],
) -> proc_macro2::TokenStream {
    let iter = variants.iter().map(|var| {
        let variant = &var.variant;
        let name = &var.name;
        let path = syn::Ident::new(variant, var.span);
        quote! {
            (#name, #ident::#path)
        }
    });

    quote! {
        impl anno_lua::AnnoEnum for #ident {
            fn variants() -> &'static [(&'static str, #ident)] {
                &[ #( #iter ),* ]
            }
        }
    }
}

fn collect_variants(
    variants: &[&Variant],
    enum_name: &str,
    use_self: bool,
) -> Result<Vec<data::Variant>, Error> {
    let mut out = vec![];
    let mut errors: Vec<Error> = vec![];

    let mut seen = HashMap::new();
    let mut n = 0;

    for variant in variants {
        let docs = collect_docs(&variant.attrs);
        let mut kv = match parse_attrs(&variant.attrs, &[("name", Kind::Name)]) {
            Ok(kv) => kv,
            Err(err) => {
                errors.push(err.into());
                continue;
            }
        };

        if let Some(span) = kv
            .iter()
            .find_map(|(k, Attr { key, .. })| (!matches!(k, Kind::Name)).then_some(key))
        {
            errors.push(Error::OnlyName(*span));
            continue;
        }

        let Attr {
            value, data: name, ..
        } = kv.remove(&Kind::Name).unwrap_or_else(|| Attr {
            key: variant.ident.span(),
            value: variant.ident.span(),
            data: variant.ident.to_string(),
        });

        let new = match &variant.fields {
            Fields::Unit if variant.discriminant.is_some() && use_self => {
                errors.push(Error::SelfDiscriminant(variant.span()));
                continue;
            }

            Fields::Unit if !use_self => {
                let discriminant = match &variant.discriminant {
                    Some((_, expr)) => {
                        let Some(t) = eval_expr(expr, &mut errors) else {
                            continue;
                        };
                        data::Discriminant::Number(t)
                    }
                    None => data::Discriminant::Number(n),
                };

                n += 1;

                data::Variant {
                    span: variant.span(),
                    variant: variant.ident.to_string(),
                    name,
                    discriminant,
                    docs,
                }
            }

            Fields::Unit => data::Variant {
                span: variant.span(),
                variant: variant.ident.to_string(),
                name,
                discriminant: data::Discriminant::Named(enum_name.to_string()),
                docs,
            },

            _ => {
                errors.push(Error::OnlyUnitVariants(variant.span()));
                continue;
            }
        };

        if let Some(prev) = seen.insert(new.name.clone(), value) {
            let mut err = syn::Error::new(value, "duplicate name found");
            err.combine(syn::Error::new(prev, "previous used here"));
            errors.push(err.into());
            continue;
        }

        out.push(new);
    }

    errors.reverse();

    if let Some(combined) =
        errors
            .into_iter()
            .map(|err| err.into_syn_error())
            .reduce(|mut left, right| {
                left.combine(right);
                left
            })
    {
        return Err(combined.into());
    }

    Ok(out)
}

fn eval_expr(expr: &Expr, errors: &mut Vec<Error>) -> Option<isize> {
    let t = match expr {
        Expr::Lit(ExprLit {
            lit: Lit::Int(lit), ..
        }) => match lit.base10_parse::<isize>() {
            Ok(number) => number,
            Err(err) => {
                errors.push(syn::Error::new(expr.span(), err).into());
                return None;
            }
        },
        Expr::Unary(ExprUnary {
            op: UnOp::Neg(..),
            expr,
            ..
        }) => {
            let Expr::Lit(
                ExprLit {
                    lit: Lit::Int(lit), ..
                },
                ..,
            ) = &**expr
            else {
                errors.push(Error::ExpectedNumber(expr.span()));
                return None;
            };
            let n = match lit.base10_parse::<isize>() {
                Ok(number) => number,
                Err(err) => {
                    errors.push(syn::Error::new(expr.span(), err).into());
                    return None;
                }
            };
            -n
        }
        _ => {
            errors.push(Error::ExpectedNumber(expr.span()));
            return None;
        }
    };
    Some(t)
}
