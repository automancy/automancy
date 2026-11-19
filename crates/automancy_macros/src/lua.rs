use core::fmt::Write;

use macro_string::MacroString;
use quote::{ToTokens, quote};
use syn::{
    Attribute, Expr, FnArg, ImplItem, ItemFn, ItemImpl, Lit, Meta, MetaNameValue, Pat, PatLit, PatTuple, PatType, Path, Signature, Token,
    Type, TypeTuple, parse,
    parse::Parse,
    parse_macro_input, parse_quote,
    punctuated::Punctuated,
    token::{Colon, Comma, Paren},
};

mod kw {
    use syn::custom_keyword;

    custom_keyword!(Result);
    custom_keyword!(mlua);
}

fn write_doc(doc: &mut String, attrs: &[Attribute]) {
    for attr in attrs {
        if attr.path().is_ident("doc") {
            let string = match &attr.meta {
                Meta::NameValue(MetaNameValue {
                    value: Expr::Lit(PatLit {
                        lit: Lit::Str(literal), ..
                    }),
                    ..
                }) => literal.value(),
                Meta::NameValue(MetaNameValue {
                    value: Expr::Macro(value),
                    ..
                }) => parse::<MacroString>(value.to_token_stream().into()).unwrap().eval().unwrap(),
                _ => panic!("{:?}", attr.meta),
            };
            let comment = string
                .split("\n")
                .map(|v| format!("---{}", v.trim()))
                .collect::<Vec<_>>()
                .join("\n")
                .replace("{", "{{")
                .replace("}", "}}");

            writeln!(doc, "{comment}").unwrap();
        }
    }
}

fn parse_func(
    doc: &mut String,
    sig: &Signature,
    input_names: &mut Vec<String>,
    input_types: &mut Vec<PatType>,
    output_type: &mut Option<Path>,
) {
    for input in sig.inputs.iter().skip(1) {
        match input {
            FnArg::Typed(input) => {
                match &*input.ty {
                    Type::Tuple(ty) => {
                        let Pat::Tuple(pat) = &*input.pat else {
                            panic!("function arg must be named with a tuple of literals")
                        };

                        for (pat, ty) in pat.elems.iter().zip(&ty.elems) {
                            let Pat::Ident(pat) = &pat else {
                                panic!("function arg must be named with a literal")
                            };

                            let Type::Path(ty) = &ty else {
                                panic!("function arg must be named with a literal")
                            };

                            let name_str = pat.ident.to_string();

                            input_names.push(name_str.clone());
                            input_types.push(PatType {
                                attrs: Vec::new(),
                                pat: Box::new(Pat::Ident(pat.clone())),
                                colon_token: Colon::default(),
                                ty: Box::new(Type::Path(ty.clone())),
                            });
                            writeln!(doc, "---@param {name_str} {{}}").unwrap();
                        }
                    },
                    Type::Path(ty) => {
                        let Pat::Ident(pat) = &*input.pat else {
                            panic!("function arg must be named with a literal")
                        };

                        let name_str = pat.ident.to_string();

                        input_names.push(name_str.clone());
                        input_types.push(PatType {
                            attrs: Vec::new(),
                            pat: Box::new(Pat::Ident(pat.clone())),
                            colon_token: Colon::default(),
                            ty: Box::new(Type::Path(ty.clone())),
                        });
                        writeln!(doc, "---@param {name_str} {{}}").unwrap();
                    },
                    _ => panic!("unsupported function arg {}", input.to_token_stream()),
                };
            },
            FnArg::Receiver(_) => panic!("macro should not be called on a method"),
        }
    }

    match &sig.output {
        syn::ReturnType::Default => {},
        syn::ReturnType::Type(_, ty) => match &**ty {
            Type::Path(ty) => {
                enum OutputPath {
                    StdResult(Path),
                    MLua(Path),
                }

                impl Parse for OutputPath {
                    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
                        let lookahead = input.lookahead1();

                        if lookahead.peek(kw::mlua) {
                            let _kw = input.parse::<kw::mlua>()?;
                            let _kw = input.parse::<Token![::]>()?;

                            let lookahead = input.lookahead1();
                            if lookahead.peek(kw::Result) {
                                let _kw = input.parse::<kw::Result>()?;
                                let _kw = input.parse::<Token![<]>()?;
                                let path = input.parse::<Path>()?;
                                let _kw = input.parse::<Token![>]>()?;

                                Ok(OutputPath::MLua(path))
                            } else {
                                Err(lookahead.error())
                            }
                        } else if lookahead.peek(kw::Result) {
                            let _kw = input.parse::<kw::Result>()?;
                            let _kw = input.parse::<Token![<]>()?;
                            let path = input.parse::<Path>()?;
                            let _kw = input.parse::<Token![,]>()?;
                            let _path = input.parse::<Path>()?;
                            let _kw = input.parse::<Token![>]>()?;

                            Ok(OutputPath::StdResult(path))
                        } else {
                            Err(lookahead.error())
                        }
                    }
                }

                if let Ok(ty) = parse::<OutputPath>(ty.to_token_stream().into()) {
                    match ty {
                        OutputPath::StdResult(path) | OutputPath::MLua(path) => {
                            *output_type = Some(path);
                            writeln!(doc, "---@return {{}}").unwrap();
                        },
                    }
                }
            },
            _ => panic!("unsupported function arg {}", ty.to_token_stream()),
        },
    }
}

/// Turn all the inputs after the mlua::Lua arg into a tuple, so it implements mlua::MultiValue
fn unwrap_func_inputs(inputs: &mut Punctuated<FnArg, Comma>, input_types: impl Iterator<Item = PatType>) {
    for _ in 0..(inputs.len() - 1) {
        inputs.pop();
    }

    let mut new_pat = PatTuple {
        attrs: Vec::new(),
        paren_token: Paren::default(),
        elems: Punctuated::new(),
    };
    let mut new_ty = TypeTuple {
        attrs: Vec::new(),
        paren_token: Paren::default(),
        elems: Punctuated::new(),
    };
    for pat_type in input_types {
        new_pat.elems.push(*pat_type.pat);
        new_ty.elems.push(*pat_type.ty);
    }

    inputs.push(FnArg::Typed(PatType {
        attrs: Vec::new(),
        pat: Box::new(Pat::Tuple(new_pat)),
        colon_token: Colon::default(),
        ty: Box::new(Type::Tuple(new_ty)),
    }));
}

pub fn lua_class(_attr: proc_macro::TokenStream, item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let mut item = parse_macro_input!(item as ItemImpl);

    let mut register_class_items = proc_macro2::TokenStream::new();
    let struct_name = match &*item.self_ty {
        Type::Path(path) => &path.path.segments.last().unwrap().ident,
        _ => panic!("lua_class must be called on a impl block on a named struct"),
    };
    let struct_name_string = struct_name.to_string();
    let struct_name_without_class = struct_name_string.trim_end_matches("Class");

    let mut class_pre = String::from("\n");
    write_doc(&mut class_pre, &item.attrs);

    // process special functions first
    let mut extra_doc_idx = None;
    let mut extra_doc_func = None;
    for (idx, func_item) in item.items.iter().enumerate().flat_map(|(idx, item)| match item {
        ImplItem::Fn(func_item) => Some((idx, func_item)),
        _ => None,
    }) {
        let func_name = &func_item.sig.ident;

        if func_name == "extra_doc" {
            assert!(extra_doc_idx.is_none());
            assert!(extra_doc_func.is_none());
            extra_doc_idx = Some(idx);
            extra_doc_func = Some(func_item.clone());
        }
    }

    if let Some(idx) = extra_doc_idx {
        item.items.remove(idx);
    }
    let extra_doc_func = extra_doc_func.map(|func| {
        let ident = &func.sig.ident;

        quote! {
            {
                #func

                type_def += #ident();
            }
        }
    });

    for func_item in item.items.iter_mut().flat_map(|item| match item {
        ImplItem::Fn(item) => Some(item),
        _ => None,
    }) {
        let func_name = func_item.sig.ident.clone();
        let func_name_string = func_name.to_string();

        let mut doc = String::from("\n");
        let mut input_names = vec![];
        let mut input_types = vec![];
        let mut output_type = None;

        {
            write_doc(&mut doc, &func_item.attrs);
            parse_func(&mut doc, &func_item.sig, &mut input_names, &mut input_types, &mut output_type);
            write!(
                &mut doc,
                "function {struct_name}.{func_name_string}({}) end",
                input_names.join(", ")
            )
            .unwrap();
        }

        let type_docs = input_types
            .iter()
            .map(|pat_type| {
                let Type::Path(ty) = &*pat_type.ty else {
                    panic!("function arg must be named with a literal")
                };
                let path = &ty.path;

                quote! {
                    <#path as automancy_data::lua::doc::LuaTypeName>::lua_name(),
                }
            })
            .chain(output_type.map(|path| {
                quote! {
                    <#path as automancy_data::lua::doc::LuaTypeName>::lua_name(),
                }
            }))
            .collect::<proc_macro2::TokenStream>();
        unwrap_func_inputs(&mut func_item.sig.inputs, input_types.into_iter());

        register_class_items.extend(quote! {
            {
                class.raw_set(
                    #func_name_string,
                    lua.create_function(#struct_name::#func_name)?
                )?;

                use core::fmt::Write;
                writeln!(type_def, #doc, #type_docs).unwrap();
            }
        });
    }

    for const_item in item.items.iter_mut().flat_map(|item| match item {
        ImplItem::Const(item) => Some(item),
        _ => None,
    }) {
        let const_name = const_item.ident.clone();
        let const_name_string = const_name.to_string().to_lowercase();

        let mut doc = String::from("\n");

        {
            let Type::Path(path) = &const_item.ty else {
                panic!("function arg must be named with a literal")
            };

            write_doc(&mut doc, &const_item.attrs);
            writeln!(&mut doc, "---@type {{}}").unwrap();
            write!(&mut doc, "{struct_name}.{const_name_string} = nil").unwrap();

            register_class_items.extend(quote! {
                {
                    class.raw_set(#const_name_string, Self::#const_name)?;

                    use core::fmt::Write;
                    writeln!(type_def, #doc, <#path as automancy_data::lua::doc::LuaTypeName>::lua_name()).unwrap();
                }
            });
        }
    }

    writeln!(&mut class_pre, "---@class (exact) {struct_name}").unwrap();
    writeln!(&mut class_pre, "local {struct_name} = {{}}").unwrap();

    let mut class_post = String::from("\n");
    writeln!(&mut class_post, "return {struct_name}").unwrap();

    item.items.push(parse_quote!(
        pub const MODULE_NAME: &str = #struct_name_without_class;
    ));

    item.items.push(parse_quote! {
        #[inline]
        #[track_caller]
        pub fn register_class(lua: &mlua::Lua) -> mlua::Result<mlua::Table> {
            use core::fmt::Write;

            let mut type_def = String::from(crate::scripting_lua::module::HEADER);
            writeln!(&mut type_def, "---@meta {}\n", #struct_name_without_class).unwrap();

            #extra_doc_func

            let class = lua.create_table()?;
            {
                type_def += #class_pre;

                {
                    #register_class_items
                }

                class.set_metatable(Some(crate::scripting_lua::create_readonly_metatable(lua, class.clone())?))?;
                lua.register_module(#struct_name_without_class, class.clone())?;

                type_def += #class_post;
            }

            {
                use std::io::Write;

                std::fs::File::create(
                    std::path::Path::new(crate::resources::LUA_TYPE_DEF_FOLDER)
                        .join(#struct_name_without_class)
                        .with_extension(crate::resources::LUA_TYPE_DEF_EXT)
                )?.write_all(type_def.as_bytes())?;
            }

            Ok(class)
        }
    });

    item.into_token_stream().into()
}

pub fn lua_global_fn(_attr: proc_macro::TokenStream, item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let mut func_item = parse_macro_input!(item as ItemFn);

    let func_name = func_item.sig.ident.clone();
    let func_name_string = func_name.to_string();

    let mut doc = String::from("\n");
    let mut input_names = vec![];
    let mut input_types = vec![];
    let mut output_type = None;

    {
        write_doc(&mut doc, &func_item.attrs);
        parse_func(&mut doc, &func_item.sig, &mut input_names, &mut input_types, &mut output_type);
        writeln!(&mut doc, "function {func_name_string}({}) end", input_names.join(", ")).unwrap();
    }

    let type_docs = input_types
        .iter()
        .map(|pat_type| {
            let Type::Path(ty) = &*pat_type.ty else {
                panic!("function arg must be named with a literal")
            };
            let path = &ty.path;

            quote! {
                <#path as automancy_data::lua::doc::LuaTypeName>::lua_name(),
            }
        })
        .chain(output_type.map(|path| {
            quote! {
                <#path as automancy_data::lua::doc::LuaTypeName>::lua_name(),
            }
        }))
        .collect::<proc_macro2::TokenStream>();
    unwrap_func_inputs(&mut func_item.sig.inputs, input_types.into_iter());

    quote! {
        {
            #func_item

            lua.globals().raw_set(
                #func_name_string,
                lua.create_function(#func_name)?
            )?;

            {
                use core::fmt::Write;
                writeln!(type_def, #doc, #type_docs).unwrap();
            }
        }
    }
    .into()
}
