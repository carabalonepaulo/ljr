use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};
use syn::Ident;

fn gen_return_value(last_letter: char) -> TokenStream {
    let mut values = vec![];
    ('A'..=last_letter)
        .map(|c| c.to_lowercase())
        .for_each(|lower| {
            let span = Span::call_site();
            let ident = format_ident!("{}_value", lower.to_string(), span = span);
            values.push(quote!(unsafe { #ident.unwrap_unchecked() }));
        });
    quote! {
        Some((#(#values,)*))
    }
}

fn gen_cast(letter: char) -> TokenStream {
    let uc = letter.to_uppercase();
    let lc = letter.to_lowercase();

    let var_ident = Ident::new(&format!("{}_value", lc), Span::call_site());
    let ty_ident = Ident::new(&uc.to_string(), Span::call_site());

    quote! {
        let #var_ident = <#ty_ident as crate::from_lua::FromLua>::from_lua(ptr, idx);
        if #var_ident.is_none() {
            return None;
        } else {
            idx += 1;
        }
    }
}

pub fn generate_from_lua_tuple_impl(_: TokenStream) -> TokenStream {
    let max = 26;
    let mut impls = vec![];
    let alphabet: Vec<char> = (b'A'..b'Z').map(|c| c as char).collect();

    (2..max).for_each(|n| {
        let mut letters_a = vec![];
        let mut where_ch = vec![];
        let mut cast_impl = vec![];
        let len = proc_macro2::Literal::i32_unsuffixed(n as i32);

        (0..n).for_each(|i| {
            let letter = alphabet[i];
            let ch = Ident::new(&letter.to_string(), Span::call_site());

            letters_a.push(ch.clone());
            cast_impl.push(gen_cast(letter));
            where_ch.push(quote!(#ch: FromLua<Output = #ch>));
        });

        let return_value = gen_return_value(alphabet[n - 1]);
        let letters_b = letters_a.clone();
        let letters_c = letters_a.clone();
        impls.push(quote! {
            impl<#(#letters_a,)*> FromLua for (#(#letters_b,)*)
            where
                #(#where_ch,)*
            {
                type Output = (#(#letters_c,)*);

                fn from_lua(ptr: *mut crate::sys::lua_State, idx: i32) -> Option<Self::Output> {
                    let top = unsafe { crate::sys::lua_gettop(ptr) };
                    let mut idx = {
                        if idx.is_negative() {
                            top + idx + 1
                        } else {
                            idx
                        }
                    };

                    if top < Self::len() {
                        return None;
                    }

                    #(#cast_impl)*

                    #return_value
                }

                fn len() -> i32 { #len }
            }
        });
    });

    quote!(#(#impls)*)
}

pub fn generate_to_lua_tuple_impl(_attr: TokenStream) -> TokenStream {
    // let max_lit = syn::parse_macro_input!(attr as LitInt);
    // let max = max_lit.base10_parse::<usize>().unwrap() + 1;
    let max = 26;
    let mut parts = vec![];
    let alphabet: Vec<char> = (b'A'..b'Z').map(|c| c as char).collect();

    (2..max).for_each(|n| {
        let mut state_push = vec![];
        let mut letters_a = vec![];
        let mut letters_b = vec![];
        let mut where_ch = vec![];
        let len = proc_macro2::Literal::i32_unsuffixed(n as i32);

        (0..n).for_each(|i| {
            let letter = alphabet[i];
            let index = syn::Index::from(i);
            let ch = Ident::new(&letter.to_string(), Span::call_site());

            // state_push.push(quote!(state.push(self.#index);));
            state_push.push(quote! { self.#index.to_lua(ptr); });
            letters_a.push(ch.clone());
            letters_b.push(ch.clone());
            where_ch.push(quote!(#ch: ToLua))
        });

        /*
        impl<A, B, C, D, E> ToLua for (A, B, C, D, E)
        where
            A: ToLua,
            B: ToLua,
            C: ToLua,
            D: ToLua,
            E: ToLua,
        {
            fn to_lua(self, state: *mut sys::lua_State) {
                todo!()
            }
        }
        */
        parts.push(quote! {
            impl<#(#letters_a,)*> ToLua for (#(#letters_b,)*)
            where
                #(#where_ch,)*
            {
                fn to_lua(self, ptr: *mut crate::sys::lua_State) {
                    #(#state_push)*
                }

                fn len() -> i32 { #len }
            }
        });
    });

    quote!(#(#parts)*)
}
