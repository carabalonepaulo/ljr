use proc_macro2::TokenStream;
use quote::quote;

pub fn module(_attr: TokenStream, item: TokenStream) -> TokenStream {
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

    let expanded = quote! {
        #func

        #[unsafe(no_mangle)]
        pub extern "C" fn #wrapper_ident(ptr: *mut ljr::sys::lua_State)
            -> ::std::os::raw::c_int
        {
            let mut lua = ljr::lua::Lua::from_ptr(ptr);
            #impl_body
        }
    };

    expanded.into()
}
