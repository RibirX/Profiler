#[proc_macro_attribute]
pub fn instrument(
  _: proc_macro::TokenStream,
  tokens: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
  tokens
}
