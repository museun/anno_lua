use syn::{parse_macro_input, spanned::Spanned, DeriveInput};

#[proc_macro_derive(Anno, attributes(anno))]
pub fn anno(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match &input.data {
        syn::Data::Struct(data) => structs::parse(&input, data),
        syn::Data::Enum(data) => enums::parse(&input, data),
        syn::Data::Union(..) => error::Error::Union(input.span()).into_compile_error(),
    }
}

mod attrs;
mod data;
mod docs;
mod enums;
mod error;
mod structs;
