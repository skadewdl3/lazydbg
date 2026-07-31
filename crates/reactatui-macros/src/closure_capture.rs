use std::collections::HashSet;

use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{ExprClosure, Ident, Token};

enum CaptureKind {
    Move,
    Borrow,
    BorrowMut,
    Clone,
}

struct Capture {
    kind: CaptureKind,
    ident: Ident,
}

pub struct ClosureInput {
    captures: Vec<Capture>,
    closure: ExprClosure,
}

impl Parse for ClosureInput {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let mut captures = Vec::new();
        let mut names = HashSet::new();

        while !input.peek(Token![|]) && !input.peek(Token![move]) {
            let kind = if input.peek(Token![&]) {
                input.parse::<Token![&]>()?;
                if input.peek(Token![mut]) {
                    input.parse::<Token![mut]>()?;
                    CaptureKind::BorrowMut
                } else {
                    CaptureKind::Borrow
                }
            } else if input.peek(Token![+]) {
                input.parse::<Token![+]>()?;
                CaptureKind::Clone
            } else {
                CaptureKind::Move
            };
            let ident = input.parse::<Ident>()?;
            if !names.insert(ident.to_string()) {
                return Err(syn::Error::new_spanned(
                    ident,
                    "a variable can only appear once in a closure capture list",
                ));
            }
            captures.push(Capture { kind, ident });
            input.parse::<Token![,]>()?;
        }

        let closure = input.parse::<ExprClosure>()?;
        if !input.is_empty() {
            return Err(input.error("unexpected tokens after closure"));
        }
        if closure.capture.is_some() {
            return Err(syn::Error::new_spanned(
                &closure,
                "do not write `move`; lambda! makes the closure a move closure",
            ));
        }

        Ok(Self { captures, closure })
    }
}

pub fn expand(input: ClosureInput) -> TokenStream2 {
    let bindings = input.captures.iter().map(|capture| {
        let ident = &capture.ident;
        match capture.kind {
            CaptureKind::Move => quote! { let #ident = #ident; },
            CaptureKind::Borrow => quote! { let #ident = &#ident; },
            CaptureKind::BorrowMut => quote! { let #ident = &mut #ident; },
            CaptureKind::Clone => {
                quote! { let #ident = ::core::clone::Clone::clone(&#ident); }
            }
        }
    });
    let closure = input.closure;

    quote! {{
        #(#bindings)*
        move #closure
    }}
}

#[cfg(test)]
mod tests {
    use quote::quote;

    use super::ClosureInput;

    #[test]
    fn rejects_duplicate_captures() {
        let error = syn::parse2::<ClosureInput>(quote! { value, +value, || {} })
            .err()
            .expect("duplicate capture should fail");

        assert!(error.to_string().contains("only appear once"));
    }

    #[test]
    fn rejects_an_explicit_move_closure() {
        let error = syn::parse2::<ClosureInput>(quote! { +value, move || {} })
            .err()
            .expect("explicit move should fail");

        assert!(error.to_string().contains("do not write `move`"));
    }
}
