use proc_macro2::{Literal, Span, TokenStream};
use quote::quote;
use syn::Ident;

use crate::parse_literal;

/**
Derives an Id registry with a convenient constructor that interns static strings as Ids.

Examples:
```
use automancy_macros::IdReg;
use automancy_data::id::Id;

#[derive(IdReg)]
pub struct FooIds {
    id_foo: Id,
    #[namespace("core")]
    id_bar: Id,
    #[namespace("meowzer/")]
    #[name("zoo")]
    id_zoo: Id,
}
```

Invalid usages:
```compile_fail
#[derive(automancy_macros::IdReg)]
pub enum Foo {}

#[derive(automancy_macros::IdReg)]
pub struct Bar();
```
*/
pub fn derive_id_reg(item: TokenStream) -> TokenStream {
    let ast: syn::DeriveInput = syn::parse2(item).unwrap();

    let namespace_ident = Ident::new("namespace", Span::call_site());
    let name_ident = Ident::new("name", Span::call_site());

    let mut names = vec![];
    let mut namespaces = vec![];

    match ast.data {
        syn::Data::Struct(data_struct) => match &data_struct.fields {
            syn::Fields::Named(fields_named) => {
                for field in fields_named.named.iter() {
                    let mut iter = field.attrs.iter();
                    let attrs @ [a, b] = [iter.next(), iter.next()];

                    let [a_ident, b_ident] = attrs.map(|v| v.and_then(|v| v.path().get_ident().cloned()));

                    let name = field.ident.clone().unwrap();

                    namespaces.push(if Some(&namespace_ident) == a_ident.as_ref() {
                        (name.clone(), parse_literal(a.unwrap()).into_iter().next().unwrap())
                    } else if Some(&namespace_ident) == b_ident.as_ref() {
                        (name.clone(), parse_literal(b.unwrap()).into_iter().next().unwrap())
                    } else {
                        (name.clone(), Literal::string("automancy"))
                    });

                    names.push(if Some(&name_ident) == a_ident.as_ref() {
                        parse_literal(a.unwrap()).into_iter().next().unwrap()
                    } else if Some(&name_ident) == b_ident.as_ref() {
                        parse_literal(b.unwrap()).into_iter().next().unwrap()
                    } else {
                        Literal::string(&name.to_string())
                    });
                }
            },
            _ => panic!("IdReg must be a struct with named fields"),
        },
        _ => panic!("IdReg must be a struct"),
    }

    let name = ast.ident;

    let items = namespaces
        .into_iter()
        .zip(names)
        .map(|((field, namespace), name)| {
            (
                field,
                format!(
                    "{}{}{}",
                    namespace.to_string().trim_matches('\"'),
                    automancy_data::id::deserialize::DELIM,
                    name.to_string().trim_matches('\"'),
                ),
            )
        })
        .flat_map(|(field, id)| {
            quote! {
                #field: interner.get_or_intern_static(#id).into(),
            }
        })
        .collect::<TokenStream>();

    quote! {
        impl #name {
            pub fn new(interner: &mut automancy_data::id::IdInterner) -> Self {
                Self {
                    #items
                }
            }
        }
    }
}
