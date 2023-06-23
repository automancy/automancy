use proc_macro::{TokenStream, TokenTree};

#[proc_macro]
pub fn make_ids(item: TokenStream) -> TokenStream {
    let items = item
        .into_iter()
        .flat_map(|v| match v {
            TokenTree::Ident(i) => Some(format!(
                "{i}: id_static(\"automancy\", \"{i}\").to_id(interner),"
            )),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        "Self {{\
            {items}\
        }}"
    )
    .parse()
    .unwrap()
}
