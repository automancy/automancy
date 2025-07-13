use proc_macro2::{Literal, TokenTree};
use syn::{Attribute, Meta};

fn parse_literal(attr: &Attribute) -> Vec<Literal> {
    match &attr.meta {
        Meta::List(list) => Some(
            list.tokens
                .clone()
                .into_iter()
                .flat_map(|v| match v {
                    TokenTree::Literal(v) => Some(v),
                    _ => None,
                })
                .collect::<Vec<_>>(),
        ),
        _ => None,
    }
    .expect("should be a literal")
}

mod ids;

#[proc_macro_derive(IdReg, attributes(name, namespace))]
pub fn derive_id_reg(tokens: proc_macro::TokenStream) -> proc_macro::TokenStream {
    ids::derive_id_reg(tokens.into()).into()
}
