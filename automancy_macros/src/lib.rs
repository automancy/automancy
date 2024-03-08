use proc_macro::TokenStream;

use proc_macro2::TokenTree;
use syn::{Attribute, Meta};

fn parse_meta(attr: &Attribute) -> Vec<String> {
    match &attr.meta {
        Meta::List(list) => Some(
            list.tokens
                .clone()
                .into_iter()
                .flat_map(|v| match v {
                    TokenTree::Literal(v) => Some(v.to_string().trim_matches('\"').to_string()),
                    TokenTree::Ident(v) => Some(v.to_string()),
                    _ => None,
                })
                .collect::<Vec<_>>(),
        ),
        _ => None,
    }
    .unwrap_or_else(|| panic!("should be either an identifier or a string in quotes"))
}

mod cell_getter;
mod ids;

#[proc_macro_derive(IdReg, attributes(name, namespace))]
pub fn derive_id_reg(item: TokenStream) -> TokenStream {
    ids::derive_id_reg(item)
}

#[proc_macro_derive(OptionGetter, attributes(getters))]
pub fn derive_option_getter(item: TokenStream) -> TokenStream {
    cell_getter::derive_option_getter(item)
}
