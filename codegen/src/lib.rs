pub mod tuple_impl;
pub mod module;

use proc_macro2::{Span, TokenStream, TokenTree};
use quote::{format_ident, quote, quote_spanned};
use syn::LitInt;
use venial::{FnParam, TypeExpr, parse_item};

fn string_to_cstr_lit(value: String) -> TokenStream {
    let buf = value.as_bytes();
    let mut nul_terminated = buf.to_vec();
    nul_terminated.push(0);
    let lit = syn::LitByteStr::new(&nul_terminated, Span::call_site());
    quote! { #lit }
}

fn type_expr_to_string(value: &TypeExpr) -> String {
    let mut s = String::new();
    for token in &value.tokens {
        s.push_str(&token.to_string());
    }
    s
}

fn type_expr_is_ref(ty: &TypeExpr) -> bool {
    match ty.tokens.first() {
        Some(TokenTree::Punct(p)) if p.as_char() == '&' => true,
        _ => false,
    }
}

fn is_type(ty: &TypeExpr, expected: &str) -> bool {
    if let Some(p) = ty.as_path() {
        if let Some(seg) = p.segments.last() {
            if seg.ident == expected {
                return true;
            }
        }
    }
    return false;
}

fn strip_ref(ty: &TypeExpr) -> Option<(TypeExpr, bool)> {
    let mut iter = ty.tokens.iter().peekable();

    let first = iter.next()?;
    match first {
        TokenTree::Punct(p) if p.as_char() == '&' => {
            let mut is_mut = false;
            if let Some(TokenTree::Ident(id)) = iter.peek() {
                if id.to_string() == "mut" {
                    is_mut = true;
                    iter.next();
                }
            }

            let rest = iter.cloned().collect();
            Some((TypeExpr { tokens: rest }, is_mut))
        }
        _ => None,
    }
}

macro_rules! try_or_return {
    ($item:ident, $($body:tt)*) => {
        match { $($body)* } {
            Some(v) => v,
            None => return $item,
        }
    };
}

pub fn generate_user_data(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let parsed_item = try_or_return!(item, parse_item(item.clone()).ok());
    let impl_block = try_or_return!(item, parsed_item.as_impl());

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
        let fn_sym = &m.name;
        let mut call_args: Vec<TokenStream> = vec![];
        let method_name = string_to_cstr_lit(m.name.to_string());
        let mut borrow_steps: Vec<TokenStream> = vec![];

        let arg_c = {
            let len_expr_list: Vec<TokenStream> = m.params.iter()
                .filter_map(|p| {
                    match &p.0 {
                        FnParam::Receiver(_) => Some(quote! { <#ud_ty as ljr::from_lua::FromLua>::len() }),
                        FnParam::Typed(ty) => {
                            let arg_ty = &ty.ty;

                            let is_ref = type_expr_is_ref(arg_ty);
                            if is_ref {
                                if let Some((inner, _)) = strip_ref(arg_ty) {
                                    if is_type(&inner, "Lua") {
                                        None
                                    } else if is_type(&inner, "str") {
                                        Some(quote! { <ljr::stack_str::StackStr as ljr::from_lua::FromLua>::len() })
                                    } else {
                                        Some(quote! { <#inner as ljr::from_lua::FromLua>::len() })
                                    }
                                } else {
                                    None
                                }
                            } else {
                                Some(quote! { <#arg_ty as ljr::from_lua::FromLua>::len() })
                            }
                        },
                    }
                }).collect();
            
            if len_expr_list.is_empty() {
                quote! { 0 }
            } else {
                quote! { (#(#len_expr_list)+*) as usize }
            }
        };

        for param in m.params.iter() {
            match &param.0 {
                FnParam::Typed(ty) => {
                    let arg_name = &ty.name;
                    let arg_ty = &ty.ty;
                    let arg_name_str = type_expr_to_string(arg_ty);
                    let is_ref = type_expr_is_ref(arg_ty);

                    if !is_ref {
                        call_args.push(quote_spanned! { arg_name.span() => #arg_name });
                        borrow_steps.push(quote_spanned! { arg_ty.span() =>
                            let #arg_name = ljr::helper::from_lua::<#arg_ty>(ptr, &mut idx, #arg_name_str);
                        })
                    } else {
                        if let Some((inner_ty, is_mut)) = strip_ref(arg_ty) {
                            let (let_def, lua_ref) = if is_mut {
                                (quote! { let mut }, quote! { &mut })
                            } else {
                                (quote! { let }, quote! { & })
                            };
                            if is_type(&inner_ty, "Lua") {
                                call_args.push(quote_spanned! { arg_name.span() => #lua_ref #arg_name });
                                borrow_steps.push(quote_spanned! { arg_ty.span() =>
                                    #let_def #arg_name = ljr::lua::Lua::from_ptr(ptr);
                                });
                            } else if is_type(&inner_ty, "str") {
                                call_args.push(quote_spanned! { arg_name.span() => #arg_name.as_str() });
                                borrow_steps.push(quote_spanned! { arg_ty.span() =>
                                    let #arg_name = ljr::helper::from_lua::<ljr::stack_str::StackStr>(ptr, &mut idx, "&str");
                                });
                            } else if is_type(&inner_ty, "StackFn") {
                                call_args.push(quote_spanned! { arg_name.span() => &#arg_name });
                                borrow_steps.push(quote_spanned! { arg_ty.span() =>
                                    let #arg_name = ljr::helper::from_lua::<#inner_ty>(ptr, &mut idx, #arg_name_str);
                                })
                            } else {
                                let (let_def, borrow_method, to_ref) = if is_mut {
                                    (quote! { let mut }, quote! { borrow_mut }, quote! { &mut * })
                                } else {
                                    (quote! { let }, quote! { borrow }, quote! { &* })
                                };
                                let guard_tmp_name = format_ident!("{}_guard", arg_name);
                                let arg_tmp_name = format_ident!("{}_tmp_ref", arg_name);

                                call_args.push(quote_spanned! { arg_name.span() => #arg_name });
                                borrow_steps.push(quote_spanned! { arg_ty.span() =>
                                    let #guard_tmp_name = ljr::helper::from_lua_stack_ref::<#inner_ty>(ptr, &mut idx);
                                    #let_def #arg_tmp_name = #guard_tmp_name.#borrow_method();
                                    let #arg_name = #to_ref #arg_tmp_name;
                                });
                            }
                        }
                    }
                },
                FnParam::Receiver(ty) => {
                    let receiver_ty = quote! { #ud_ty };
                    call_args.push(quote! { __ud_ref });

                    let (let_def, borrow_method, to_ref) = if ty.tk_mut.is_some() {
                        (quote! { let mut }, quote! { borrow_mut }, quote! { &mut * })
                    } else {
                        (quote! { let }, quote! { borrow }, quote! { &* })
                    };

                    borrow_steps.push(quote! {
                        let __ud_guard = ljr::helper::from_lua_stack_ref::<#receiver_ty>(ptr, &mut idx);
                        #let_def __ud_tmp_ref = __ud_guard.#borrow_method();
                        let __ud_ref = #to_ref __ud_tmp_ref;
                    });
                }
            }
        }

        let final_block = quote! {
            ljr::helper::catch(ptr, move || {
                let mut idx = 1;

                #(#borrow_steps)*

                #ud_ty::#fn_sym(#(#call_args),*)
            })
        };

        quote! {
            ljr::sys::luaL_Reg {
                name: #method_name.as_ptr() as _,
                func: {
                    unsafe extern "C-unwind" fn trampoline(ptr: *mut ljr::sys::lua_State) -> std::ffi::c_int {
                        ljr::helper::check_arg_count(ptr, #arg_c);
                        #final_block
                    }
                    trampoline
                }
            }
        }
    });

    let mut count = 1;
    let mut reg_list = quote! {};
    for reg in regs {
        reg_list.extend(quote! { #reg, });
        count += 1;
    }

    let regs_ident = format_ident!("{}_REGS", ud_name.to_uppercase());
    let regs_count = LitInt::new(format!("{}", count).as_str(), Span::call_site());

    quote! {
        #item

        const #regs_ident: [ljr::sys::luaL_Reg; #regs_count] = [
            #reg_list
            ljr::sys::luaL_Reg {
                name: std::ptr::null(),
                func: ljr::dummy_trampoline,
            }
        ];

        impl ljr::UserData for #ud_ty {
            fn name() -> *const i8 {
                #ud_ty_name.as_ptr() as _
            }

            fn functions() -> &'static [ljr::sys::luaL_Reg] {
                &#regs_ident
            }
        }
    }
}
