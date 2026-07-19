use proc_macro2::TokenStream;
use quote::quote;
use syn::{Expr, ExprCall, ItemFn, Stmt, Token, parse_quote, parse_str, parse2 as parse};

pub fn thingy_impl(input: TokenStream) -> TokenStream {
    let mut func = parse::<ItemFn>(input).unwrap();
    func.block.stmts.push(parse_quote! {
        reactatui::hello_world();
    });
    quote!(#func).into()
}
