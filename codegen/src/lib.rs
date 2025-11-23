pub mod tuple_impl;
pub mod module;
mod type_info;

use proc_macro2::{Span, TokenStream, TokenTree};
use quote::{format_ident, quote, quote_spanned};
use syn::{LitInt};
use venial::{FnParam, parse_item};

use crate::type_info::{Ref, TypeInfo};

const SPECIAL_TYPES: [&'static str; 6] = ["StackStr", "StackFn", "StackTable", "LStr<Borrowed>", "Func<Borrowed", "Table<Borrowed>"];

fn string_to_cstr_lit(value: String) -> TokenStream {
    let buf = value.as_bytes();
    let mut nul_terminated = buf.to_vec();
    nul_terminated.push(0);
    let lit = syn::LitByteStr::new(&nul_terminated, Span::call_site());
    quote! { #lit }
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
        let mut safe_args: Vec<TokenStream> = vec![];

        let arg_c = {
            let len_expr_list: Vec<TokenStream> = m.params.iter()
                .filter_map(|p| {
                    match &p.0 {
                        FnParam::Receiver(_) => Some(quote! { <#ud_ty as ljr::from_lua::FromLua>::len() }),
                        FnParam::Typed(ty) => {
                            let arg_ty = &ty.ty;
                            let type_info = TypeInfo::new(arg_ty);
                            let ty_name = type_info.name();
                            let inner_ty = type_info.inner_ty();

                            let is_ref = type_info.ref_kind().is_some();
                            if is_ref {
                                if ty_name == "Lua" {
                                    None
                                } else if ty_name == "str" {
                                    Some(quote! { <ljr::lstr::StackStr as ljr::from_lua::FromLua>::len() })
                                } else {
                                    Some(quote! { <#inner_ty as ljr::from_lua::FromLua>::len() })
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
                    let type_info = TypeInfo::new(&arg_ty);
                    let is_ref = type_info.ref_kind().is_some();
                    let arg_name_str = type_info.name();

                    if !is_ref {
                        for partial_ty_name in SPECIAL_TYPES.iter() {
                            if !type_info.name().starts_with(partial_ty_name) {
                                continue;
                            }
                            panic!("the type {0} cannot be taken by value, only by reference, try using &{0} or &mut {0}", type_info.name())
                        }

                        safe_args.push(quote_spanned! { arg_ty.span() => ljr::lua::ensure_get_global_impl::<#arg_ty>(); });
                        call_args.push(quote_spanned! { arg_name.span() => #arg_name });
                        borrow_steps.push(quote_spanned! { arg_ty.span() =>
                            let #arg_name = ljr::helper::from_lua::<#arg_ty>(ptr, &mut idx, #arg_name_str);
                        })
                    } else {
                        let ty_name = arg_name_str;
                        let ty_ident = type_info.inner_ty();
                        let is_mut = matches!(type_info.ref_kind(), Some(Ref::Mut));

                        let (let_def, lua_ref) = if is_mut {
                            (quote! { let mut }, quote! { &mut })
                        } else {
                            (quote! { let }, quote! { & })
                        };
                        if ty_name == "Lua" {
                            call_args.push(quote_spanned! { arg_name.span() => #lua_ref #arg_name });
                            borrow_steps.push(quote_spanned! { arg_ty.span() =>
                                #let_def #arg_name = ljr::lua::Lua::from_ptr(ptr);
                            });
                        } else if ty_name == "str" {
                            call_args.push(quote_spanned! { arg_name.span() => #arg_name.as_str().expect("lua string is not a valid rust string") });
                            borrow_steps.push(quote_spanned! { arg_ty.span() =>
                                let #arg_name = ljr::helper::from_lua::<ljr::lstr::StackStr>(ptr, &mut idx, "&str");
                            });
                        } else if SPECIAL_TYPES.iter().any(|n| type_info.name().starts_with(n)) {
                            call_args.push(quote_spanned! { arg_name.span() => #lua_ref #arg_name });
                            borrow_steps.push(quote_spanned! { arg_ty.span() =>
                                #let_def #arg_name = ljr::helper::from_lua::<#ty_ident>(ptr, &mut idx, #arg_name_str);
                            })
                        } else {
                            let (let_def, borrow_method, to_ref) = if is_mut {
                                (quote! { let mut }, quote! { as_mut }, quote! { &mut * })
                            } else {
                                (quote! { let }, quote! { as_ref }, quote! { &* })
                            };
                            let guard_tmp_name = format_ident!("{}_guard", arg_name);
                            let arg_tmp_name = format_ident!("{}_tmp_ref", arg_name);

                            call_args.push(quote_spanned! { arg_name.span() => #arg_name });
                            borrow_steps.push(quote_spanned! { arg_ty.span() =>
                                #let_def #guard_tmp_name = ljr::helper::from_lua_stack_ref::<#ty_ident>(ptr, &mut idx);
                                #let_def #arg_tmp_name = #guard_tmp_name.#borrow_method();
                                let #arg_name = #to_ref #arg_tmp_name;
                            });
                        }
                    }
                },
                FnParam::Receiver(ty) => {
                    let receiver_ty = quote! { #ud_ty };
                    call_args.push(quote! { __ud_ref });

                    let (let_def, borrow_method, to_ref) = if ty.tk_mut.is_some() {
                        (quote! { let mut }, quote! { as_mut }, quote! { &mut * })
                    } else {
                        (quote! { let }, quote! { as_ref }, quote! { &* })
                    };

                    borrow_steps.push(quote! {
                        #let_def __ud_guard = ljr::helper::from_lua_stack_ref::<#receiver_ty>(ptr, &mut idx);
                        #let_def __ud_tmp_ref = __ud_guard.#borrow_method();
                        let __ud_ref = #to_ref __ud_tmp_ref;
                    });
                }
            }
        }

        let final_block = quote! {
            ljr::helper::catch(ptr, move || {
                #(#safe_args)*

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

