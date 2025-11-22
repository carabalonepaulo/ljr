use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn user_data(attr: TokenStream, item: TokenStream) -> TokenStream {
    codegen::generate_user_data(attr.into(), item.into()).into()
}

#[proc_macro]
pub fn generate_to_lua_tuple_impl(attr: TokenStream) -> TokenStream {
    codegen::tuple_impl::generate_to_lua_tuple_impl(attr.into()).into()
}

#[proc_macro]
pub fn generate_from_lua_tuple_impl(attr: TokenStream) -> TokenStream {
    codegen::tuple_impl::generate_from_lua_tuple_impl(attr.into()).into()
}

#[proc_macro]
pub fn generate_get_global_tuple_impl(attr: TokenStream) -> TokenStream {
    codegen::tuple_impl::generate_get_global_tuple_impl(attr.into()).into()
}

#[proc_macro_attribute]
pub fn module(attr: TokenStream, item: TokenStream) -> TokenStream {
    codegen::module::module(attr.into(), item.into()).into()
}
