use proc_macro2::TokenStream;
use quote::quote;

pub fn module(attr: TokenStream, item: TokenStream) -> TokenStream {
    let result = venial::parse_item(item).unwrap();
    let func = match result {
        venial::Item::Function(f) => f,
        _ => panic!("ljr::module macro can only by user on functions"),
    };

    let name = &func.name;
    let wrapper_ident = syn::Ident::new(&format!("luaopen_{}", name.to_string()), name.span());

    let ret_ty = &func.return_ty.clone();

    let impl_body = if let Some(ty) = ret_ty {
        quote! {
            let opt_value = #name(&mut lua);
            <#ty as ljr::to_lua::ToLua>::to_lua(opt_value, ptr);
            <#ty as ljr::to_lua::ToLua>::len()
        }
    } else {
        quote! {
            #name(&mut lua);
            0
        }
    };

    let must_ensure_main = attr.to_string().contains("ensure_main_state");
    let ensure_main_state = if must_ensure_main {
        quote! {
            if let Err(e) = unsafe { lua.assert_main_state() } {
                let err_msg = e.to_string();
                let c_msg = ::std::ffi::CString::new(err_msg).unwrap_or_default();

                std::mem::drop(e);
                std::mem::drop(lua);

                unsafe {
                    ljr::sys::lua_pushstring(ptr, c_msg.as_ptr());
                    std::mem::drop(c_msg);
                    ljr::sys::lua_error(ptr);
                }
            }
        }
    } else {
        quote!()
    };

    let expanded = quote! {
        #func

        #[unsafe(no_mangle)]
        pub extern "C-unwind" fn #wrapper_ident(ptr: *mut ljr::sys::lua_State)
            -> ::std::os::raw::c_int
        {
            let mut lua = ljr::lua::Lua::from_ptr(ptr);
            #ensure_main_state
            #impl_body
        }
    };

    expanded.into()
}
