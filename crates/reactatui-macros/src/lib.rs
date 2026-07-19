use proc_macro::TokenStream;
use syn::parse_macro_input;

mod thing;

#[proc_macro_attribute]
pub fn thingy(_metadata: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input);
    thing::thingy_impl(input).into()
}
