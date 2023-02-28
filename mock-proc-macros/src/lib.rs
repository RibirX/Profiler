#[proc_macro_attribute]
pub fn instrument(
  _: proc_macro::TokenStream,
  _: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
  proc_macro::TokenStream::new()
}
