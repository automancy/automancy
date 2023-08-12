use proc_macro::TokenStream;

use syn::__private::ToTokens;

use crate::parse_meta;

const GET: &str = "get";

pub fn derive_option_getter(item: TokenStream) -> TokenStream {
    let ast: syn::DeriveInput = syn::parse(item).unwrap();

    let mut items = vec![];

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

                    let attrs = attrs.map(parse_meta).unwrap_or(vec![]);

                    let name = field
                        .ident
                        .clone()
                        .expect("somehow the field doesn't have a name")
                        .to_string();

                    if attrs.contains(&GET.to_string()) {
                        items.push((
                            name,
                            field
                                .ty
                                .to_token_stream()
                                .into_iter()
                                .nth(2) // strip away *Cell < >
                                .unwrap()
                                .to_string(),
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
        .map(|(item_name, item_type)| {
            format!(
                "
                impl {name} {{
                    pub fn {item_name}(&self) -> &{item_type} {{
                        self.{item_name}.as_ref().unwrap()
                    }}
                    pub fn {item_name}_mut(&mut self) -> &mut {item_type} {{
                        self.{item_name}.as_mut().unwrap()
                    }}
                }}
                "
            )
        })
        .collect::<String>()
        .parse()
        .unwrap()
}
