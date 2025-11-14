use proc_macro2::{Span, TokenStream, TokenTree};
use quote::quote;
use venial::{FnParam, TypeExpr, parse_item};

fn string_to_str_lit(value: String) -> TokenStream {
    let lit = syn::LitStr::new(value.as_str(), Span::call_site());
    quote! { #lit }
}

fn string_to_cstr_lit(value: String) -> TokenStream {
    let buf = value.as_bytes();
    let mut nul_terminated = buf.to_vec();
    nul_terminated.push(0);
    let lit = syn::LitByteStr::new(&nul_terminated, Span::call_site());
    quote! { #lit }
}

fn type_expr_to_string(value: &TypeExpr) -> Option<String> {
    match value.tokens.first() {
        Some(TokenTree::Ident(ident)) => Some(ident.to_string()),
        _ => None,
    }
}

pub fn generate_user_data(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let parsed_item = parse_item(item.clone()).unwrap();
    let impl_block = parsed_item.as_impl().unwrap();
    let ud_name = match impl_block.self_ty.tokens.first() {
        Some(TokenTree::Ident(ident)) => ident.to_string(),
        _ => panic!("invalid type identifier"),
    };
    let ud_ty = impl_block.self_ty.clone();
    let ud_ty_name = {
        let buf = ud_name.as_bytes();
        let mut nul_terminated = buf.to_vec();
        nul_terminated.push(0);
        syn::LitByteStr::new(&nul_terminated, Span::call_site())
    };

    let methods = impl_block.body_items.iter().filter_map(|item| match item {
        venial::ImplMember::AssocFunction(f) => Some(f),
        _ => None,
    });

    let regs = methods.map(|m| {
        let method_name = string_to_cstr_lit(m.name.to_string());
        let mut args = vec![quote! {
            let mut idx = 1;
        }];

        let mut call_args = vec![];

        for param in m.params.iter() {
            match &param.0 {
                FnParam::Typed(ty) => {
                    let arg_name = &ty.name;
                    let arg_ty = &ty.ty;
                    let arg_name_str = type_expr_to_string(&ty.ty);

                    call_args.push(quote! { #arg_name });
                    args.push(quote! {
                        let #arg_name = ljr::helper::from_lua::<#arg_ty>(ptr, &mut idx, #arg_name_str);
                    });
                }
                FnParam::Receiver(ty) => {
                    let (let_def, ref_tk) = if ty.tk_mut.is_some() {
                        (quote! { let mut }, quote! { &mut *ud })
                    } else {
                        (quote! { let }, quote! { &*ud })
                    };

                    call_args.push(quote! { ud_ref });

                    args.push(quote! {
                        #let_def ud = ljr::helper::from_lua_ref::<#ud_ty>(ptr, &mut idx);
                        idx += 1;
                        let ud_ref = #ref_tk;
                    });
                }
            }
        }

        let fn_sym = &m.name;
        let final_block = quote! {
            ljr::helper::catch(ptr, move || {
                #(#args)*;
                #ud_ty::#fn_sym(#(#call_args),*)
            })
        };

        quote! {
            sys::luaL_Reg {
                name: #method_name.as_ptr() as _,
                func: {
                    unsafe extern "C" fn trampoline(ptr: *mut sys::lua_State) -> std::ffi::c_int {
                        #final_block
                    }
                    Some(trampoline)
                }
            }
        }
    });

    let mut reg_list = quote! {};
    for reg in regs {
        reg_list.extend(quote! { #reg, });
    }

    quote! {
        #item

        impl UserData for #ud_ty {
            fn name() -> *const i8 {
                #ud_ty_name.as_ptr() as _
            }

            fn functions() -> Vec<luajit2_sys::luaL_Reg> {
                vec![
                    #reg_list
                    sys::luaL_Reg {
                        name: std::ptr::null(),
                        func: None,
                    }
                ]
            }
        }
    }
}
