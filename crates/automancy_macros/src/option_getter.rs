use proc_macro2::{Ident, Span, TokenStream, TokenTree};
use quote::{ToTokens, TokenStreamExt, format_ident, quote};

use crate::parse_ident;

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
                        .expect("somehow the field doesn't have a name");

                    if attrs.contains(&get_ident) {
                        let mut vec =
                            Vec::<TokenTree>::from_iter(field.ty.to_token_stream().into_iter());

                        // Remove Option
                        vec.remove(0);
                        // Remove <
                        vec.remove(0);
                        // Remove >
                        vec.pop();

                        let mut tokens = TokenStream::new();
                        tokens.append_all(&mut vec);

                        items.push((name, tokens));
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
