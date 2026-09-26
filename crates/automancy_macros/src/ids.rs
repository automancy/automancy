use proc_macro2::Literal;
use quote::{ToTokens, quote};
use syn::{Data, DeriveInput, Fields, Token, bracketed, parenthesized, parse, parse::Parse, parse_macro_input};

mod kw {
    use syn::custom_keyword;

    custom_keyword!(namespace);
    custom_keyword!(name);
}

enum IdRegArg {
    Namespace(Literal),
    Name(Literal),
}

impl Parse for IdRegArg {
    fn parse(attr: syn::parse::ParseStream) -> syn::Result<Self> {
        let _ = attr.parse::<Token![#]>();

        let input;
        bracketed!(input in attr);

        let lookahead = input.lookahead1();
        if lookahead.peek(kw::namespace) {
            let _kw = input.parse::<kw::namespace>()?;
            let content;
            parenthesized!(content in input);
            let namespace = content.parse::<Literal>()?;

            Ok(IdRegArg::Namespace(namespace))
        } else if lookahead.peek(kw::name) {
            let _kw = input.parse::<kw::name>()?;
            let content;
            parenthesized!(content in input);
            let name = content.parse::<Literal>()?;

            Ok(IdRegArg::Name(name))
        } else {
            Err(lookahead.error())
        }
    }
}

pub fn derive_id_reg(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = input.ident;

    let default_namespace = Literal::string("automancy");

    let mut ids = vec![];
    match input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => {
                for field in fields.named.iter() {
                    let field_name = field.ident.clone().expect("field should be named");

                    let mut namespace = None;
                    let mut name = None;

                    for arg in field
                        .attrs
                        .iter()
                        .flat_map(|attr| parse::<IdRegArg>(attr.into_token_stream().into()))
                    {
                        match arg {
                            IdRegArg::Namespace(v) => {
                                assert!(namespace.is_none(), "namespace attribute should only be present once");
                                namespace = Some(v);
                            },
                            IdRegArg::Name(v) => {
                                assert!(name.is_none(), "namespace attribute should only be present once");
                                name = Some(v);
                            },
                        }
                    }

                    ids.push((
                        namespace.unwrap_or(default_namespace.clone()),
                        name.unwrap_or(Literal::string(&field_name.to_string())),
                        field_name,
                    ))
                }
            },
            _ => panic!("IdReg must be a struct with named fields"),
        },
        _ => panic!("IdReg must be a struct"),
    }

    let items = ids
        .into_iter()
        .map(|(namespace, name, field)| {
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
        .collect::<proc_macro2::TokenStream>();

    quote! {
        impl #name {
            pub fn new(interner: &mut automancy_data::id::IdInterner) -> Self {
                Self {
                    #items
                }
            }
        }
    }
    .into()
}
