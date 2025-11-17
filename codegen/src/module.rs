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

    let expanded = quote! {
        #func

        #[unsafe(no_mangle)]
        pub extern "C" fn #wrapper_ident(ptr: *mut ljr::sys::lua_State)
            -> ::std::os::raw::c_int
        {
            let mut lua = ljr::lua::Lua::from_ptr(ptr);
            #name(&mut lua);
            0
        }
    };

    expanded.into()
}
