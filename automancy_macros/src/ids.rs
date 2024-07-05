use crate::parse_literal;
use proc_macro2::TokenStream;
use proc_macro2::{Literal, Span};
use quote::quote;
use syn::Ident;

/// # Examples
///
/// ```
/// use automancy_macros::IdReg;
/// use automancy_defs::id::Id;
///
/// #[derive(IdReg)]
/// pub struct FooIds {
///     id_foo: Id,
///     #[namespace("core")]
///     id_bar: Id,
///     #[namespace("meowzer/")]
///     #[name("zoo")]
///     id_zoo: Id,
/// }
/// ```
///
/// # Invalid usages
/// ```compile_fail
/// #[derive(automancy_macros::IdReg)]
/// pub enum Foo {}
///
/// #[derive(automancy_macros::IdReg)]
/// pub struct Bar();
/// ```
pub fn derive_id_reg(item: TokenStream) -> TokenStream {
    let ast: syn::DeriveInput = syn::parse2(item).unwrap();

    let namespace_lit = Ident::new("namespace", Span::call_site());
    let name_lit = Ident::new("name", Span::call_site());

    let mut names = vec![];
    let mut namespaces = vec![];

    match ast.data {
        syn::Data::Struct(data_struct) => match &data_struct.fields {
            syn::Fields::Named(fields_named) => {
                for field in fields_named.named.iter() {
                    let mut iter = field.attrs.iter();
                    let attrs @ [a, b] = [iter.next(), iter.next()];

                    let [a_ident, b_ident] =
                        attrs.map(|v| v.and_then(|v| v.path().get_ident().cloned()));

                    let name = field.ident.clone().unwrap();

                    namespaces.push(if Some(&namespace_lit) == a_ident.as_ref() {
                        (
                            name.clone(),
                            parse_literal(a.unwrap()).into_iter().next().unwrap(),
                        )
                    } else if Some(&namespace_lit) == b_ident.as_ref() {
                        (
                            name.clone(),
                            parse_literal(b.unwrap()).into_iter().next().unwrap(),
                        )
                    } else {
                        (name.clone(), Literal::string("automancy"))
                    });

                    names.push(if Some(&name_lit) == a_ident.as_ref() {
                        parse_literal(a.unwrap()).into_iter().next().unwrap()
                    } else if Some(&name_lit) == b_ident.as_ref() {
                        parse_literal(b.unwrap()).into_iter().next().unwrap()
                    } else {
                        Literal::string(&name.to_string())
                    });
                }
            }
            _ => panic!("must be a struct with named fields"),
        },
        _ => panic!("must be a struct"),
    }

    let name = ast.ident;

    let items = namespaces
        .into_iter()
        .zip(names)
        .flat_map(|((field, namespace), name)| {
            quote! {
                #field: automancy_defs::id::IdRaw::new(#namespace, #name).to_id(interner),
            }
        })
        .collect::<TokenStream>();

    quote! {
        impl #name {
            pub fn new(interner: &mut automancy_defs::id::Interner) -> Self {
                Self {
                    #items
                }
            }
        }
    }
}
