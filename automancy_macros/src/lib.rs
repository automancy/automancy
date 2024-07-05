use proc_macro2::Span;
use proc_macro2::{Literal, TokenStream, TokenTree};
use quote::{quote, ToTokens};

use syn::{
    parse::Parse, token::Comma, Attribute, ExprClosure, ExprLit, Ident, Meta, Pat, PatReference,
    Token, Type, TypeInfer, TypeReference,
};

fn parse_ident(attr: &Attribute) -> Vec<Ident> {
    match &attr.meta {
        Meta::List(list) => Some(
            list.tokens
                .clone()
                .into_iter()
                .flat_map(|v| match v {
                    TokenTree::Ident(v) => Some(v),
                    _ => None,
                })
                .collect::<Vec<_>>(),
        ),
        _ => None,
    }
    .unwrap_or_else(|| panic!("should be either an identifier"))
}

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
    .unwrap_or_else(|| panic!("should be either an identifier"))
}

fn pat_to_type(pat: &Pat) -> Type {
    match pat {
        Pat::Type(v) => {
            if let Type::Infer(_) = *v.ty {
                pat_to_type(&v.pat)
            } else {
                (*v.ty).clone()
            }
        }
        Pat::Reference(PatReference {
            and_token,
            mutability,
            pat,
            ..
        }) => Type::Reference(TypeReference {
            and_token: *and_token,
            lifetime: None,
            mutability: *mutability,
            elem: Box::new(pat_to_type(pat)),
        }),
        _ => Type::Infer(TypeInfer {
            underscore_token: Default::default(),
        }),
    }
}

fn pat_to_ident(pat: &Pat) -> Ident {
    match pat {
        Pat::Ident(v) => v.ident.clone(),
        Pat::Type(v) => pat_to_ident(&v.pat),
        _ => Ident::new("_", Span::call_site()),
    }
}

fn type_to_name(t: Type) -> String {
    match t {
        Type::Path(v) => v
            .path
            .segments
            .last()
            .cloned()
            .unwrap()
            .to_token_stream()
            .into_iter()
            .map(|v| v.to_string())
            .collect::<String>(),
        _ => panic!("no name found for type"),
    }
}

struct RhaiClosureInput {
    name: ExprLit,
    _c1: Token![,],
    closure: ExprClosure,
}

impl Parse for RhaiClosureInput {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Self {
            name: input.parse()?,
            _c1: input.parse()?,
            closure: input.parse()?,
        })
    }
}

mod ids;
mod option_getter;

#[proc_macro_derive(IdReg, attributes(name, namespace))]
pub fn derive_id_reg(tokens: proc_macro::TokenStream) -> proc_macro::TokenStream {
    ids::derive_id_reg(tokens.into()).into()
}

#[proc_macro_derive(OptionGetter, attributes(getters))]
pub fn derive_option_getter(tokens: proc_macro::TokenStream) -> proc_macro::TokenStream {
    option_getter::derive_option_getter(tokens.into()).into()
}

#[proc_macro]
pub fn rhai_register_closure(tokens: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = syn::parse_macro_input!(tokens as RhaiClosureInput);

    let name = match input.name.lit {
        syn::Lit::Str(str) => str.token().to_string(),
        _ => panic!("name must be a string literal"),
    };

    let closure = input.closure;
    let input_types = closure.inputs.iter().map(pat_to_type);
    let input_names = closure.inputs.iter().map(pat_to_ident);

    let params = input_types
        .zip(input_names)
        .map(|(t, n)| format!("{}: {}", n, type_to_name(t)))
        .chain(std::iter::once(match closure.output.clone() {
            syn::ReturnType::Default => panic!("return type must be explicit"),
            syn::ReturnType::Type(_, t) => type_to_name(*t),
        }))
        .flat_map(|v| {
            [
                Literal::string(&v).to_token_stream(),
                Comma(Span::call_site()).to_token_stream(),
            ]
        })
        .collect::<TokenStream>();

    quote! {
        rhai::FuncRegistration::new(#name)
            .in_internal_namespace()
            .with_purity(true)
            .with_volatility(false)
            .with_params_info(&[#params])
            .set_into_module(
                &mut module,
                #closure
            );
    }
    .into()
}
