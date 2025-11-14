use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn user_data(attr: TokenStream, item: TokenStream) -> TokenStream {
    codegen::generate_user_data(attr.into(), item.into()).into()
}
