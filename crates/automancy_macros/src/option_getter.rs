use crate::parse_ident;
use proc_macro2::TokenStream;
use proc_macro2::{Ident, Span};
use quote::{format_ident, quote, ToTokens};

pub fn derive_option_getter(item: TokenStream) -> TokenStream {
    let ast: syn::DeriveInput = syn::parse2(item).unwrap();

    let mut items = vec![];

    let get_ident = Ident::new("get", Span::call_site());

    match ast.data {
        syn::Data::Struct(data_struct) => match &data_struct.fields {
            syn::Fields::Named(fields_named) => {
                for field in fields_named.named.iter() {
                    let mut iter = field.attrs.iter();
                    let attrs = iter.find(|v| {
                        v.path()
                            .get_ident()
                            .map(|ident| *ident == "getters")
                            .unwrap_or(false)
                    });

                    let attrs = attrs.map(parse_ident).unwrap_or(vec![]);

                    let name = field
                        .ident
                        .clone()
                        .expect("Somehow the field doesn't have a name");

                    if attrs.contains(&get_ident) {
                        items.push((
                            name,
                            field
                                .ty
                                .to_token_stream()
                                .into_iter()
                                .nth(2) // strip away *Cell < >
                                .unwrap(),
                        ));
                    }
                }
            }
            _ => panic!("must be a struct with named fields"),
        },
        _ => panic!("must be a struct"),
    }

    let name = ast.ident;

    items
        .iter()
        .flat_map(|(item_name, item_type)| {
            let item_name_mut = format_ident!("{}_mut", item_name);

            quote! {

                impl #name {
                    pub fn #item_name(&self) -> &#item_type {
                        self.#item_name.as_ref().unwrap()
                    }
                    pub fn #item_name_mut(&mut self) -> &mut #item_type {
                        self.#item_name.as_mut().unwrap()
                    }
                }
            }
        })
        .collect::<TokenStream>()
}
