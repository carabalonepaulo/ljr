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

        let mut receiver_present = false;
        let mut receiver_name = format_ident!("_");
        let mut receiver_ty = quote! {};

        if let Some(_) = m
            .params
            .iter()
            .find(|p| matches!(&p.0, FnParam::Receiver(_)))
        {
            receiver_present = true;
            receiver_name = format_ident!("ud_ref");
            receiver_ty = quote! { #ud_ty };
            call_args.push(quote! { #receiver_name });
        }

        let args_c = m.params.iter()
            .filter(|p| {
                match &p.0 {
                    FnParam::Receiver(_) => true,
                    FnParam::Typed(ty) => {
                        let arg_ty = &ty.ty;

                        let is_ref = type_expr_is_ref(arg_ty);
                        if is_ref {
                            if let Some((inner, _)) = strip_ref(arg_ty) {
                                !is_type(&inner, "Lua")
                            } else {
                                false
                            }
                        } else {
                            true
                        }
                    },
                }
            }).count();

        for param in m.params.iter() {
            if let FnParam::Typed(ty) = &param.0 {
                let arg_name = &ty.name;
                let arg_ty = &ty.ty;
                let is_ref = type_expr_is_ref(arg_ty);

                if is_ref {
                    if let Some((inner_ty, _)) = strip_ref(arg_ty) {
                        if is_type(&inner_ty, "Lua") {
                            call_args.push(quote_spanned! { arg_name.span() => &#arg_name });
                        } else if is_type(&inner_ty, "str") {
                            call_args.push(quote_spanned! { arg_name.span() => #arg_name.as_str() });
                        } else {
                            call_args.push(quote_spanned! { arg_name.span() => #arg_name });
                        }
                    }
                } else {
                    call_args.push(quote_spanned! { arg_name.span() => #arg_name });
                }
            }
        }

        let mut inner_most_block = quote! {
            #ud_ty::#fn_sym(#(#call_args),*)
        };

        for param in m.params.iter().rev() {
            if let FnParam::Typed(ty) = &param.0 {
                let arg_name = &ty.name;
                let arg_ty = &ty.ty;
                let arg_name_str = type_expr_to_string(arg_ty);
                let is_ref = type_expr_is_ref(arg_ty);

                if !is_ref {
                    borrow_steps.push(quote_spanned! { arg_ty.span() =>
                        let #arg_name = ljr::helper::from_lua::<#arg_ty>(ptr, &mut idx, #arg_name_str);
                    })
                } else {
                    if let Some((inner_ty, is_mut)) = strip_ref(arg_ty) {
                        if is_type(&inner_ty, "Lua") {
                            borrow_steps.push(quote_spanned! { arg_ty.span() =>
                                let #arg_name = ljr::lua::Lua::from_ptr(ptr);
                            });
                        } else if is_type(&inner_ty, "str") {
                            borrow_steps.push(quote_spanned! { arg_ty.span() =>
                                let #arg_name = ljr::helper::from_lua::<ljr::stack_str::StackStr>(ptr, &mut idx, "&str");
                            });
                        } else {
                            let borrow_method = if is_mut { quote! { with_mut } } else { quote! { with } };
                            let arg_tmp_name = format_ident!("{}_stack_ref", arg_name);

                            borrow_steps.push(quote_spanned! { arg_ty.span() =>
                                let #arg_tmp_name = ljr::helper::from_lua_stack_ref::<#inner_ty>(ptr, &mut idx);
                            });

                            inner_most_block = quote! {
                                #arg_tmp_name.#borrow_method(|#arg_name| {
                                    #inner_most_block
                                })
                            };
                        }
                    }
                }
            }
        }

        if receiver_present {
            let (let_def, borrow_method) = if let FnParam::Receiver(ref_ty) = &m.params.iter().find(|p| matches!(&p.0, FnParam::Receiver(_))).unwrap().0 {
                if ref_ty.tk_mut.is_some() {
                    (quote! { let mut ud_stack_ref = }, quote! { with_mut })
                } else {
                    (quote! { let ud_stack_ref = }, quote! { with })
                }
            } else {
                unreachable!();
            };

            inner_most_block = quote! {
                #let_def ljr::helper::from_lua_stack_ref::<#receiver_ty>(ptr, &mut idx);
                ud_stack_ref.#borrow_method(|#receiver_name| {
                    #inner_most_block
                })
            }
        }

        borrow_steps = borrow_steps.into_iter().rev().collect();

        let final_block = quote! {
            ljr::helper::catch(ptr, move || {
                let mut idx = 1;

                #(#borrow_steps)*

                #inner_most_block
            })
        };

        let expected_args = LitInt::new(format!("{}", args_c).as_str(), Span::call_site());
        quote! {
            sys::luaL_Reg {
                name: #method_name.as_ptr() as _,
                func: {
                    unsafe extern "C" fn trampoline(ptr: *mut sys::lua_State) -> std::ffi::c_int {
                        ljr::helper::check_arg_count(ptr, #expected_args);
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
