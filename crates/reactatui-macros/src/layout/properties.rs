//! Helpers for declaring layout-property tables.
//!
//! A property module only needs to list the CSS name, target builder method,
//! and value parser. The generated enum handles CSS-name deserialization and
//! emits the corresponding [`RuleTokens`](crate::layout::RuleTokens).

macro_rules! properties {
    (
        $name:ident {
            $(
                $variant:ident => ($css:literal, $method:ident, $value:ty)
            ),* $(,)?
        }
    ) => {
        #[derive(serde::Deserialize)]
        enum $name {
            $(
                #[serde(rename = $css)]
                $variant,
            )*
        }

        impl $name {
            fn emit(self, value: proc_macro2::TokenStream) -> syn::Result<crate::layout::RuleTokens> {
                match self {
                    $(
                        Self::$variant => {
                            let value = <$value as crate::layout::CssValue>::parse_tokens(value, $css)?;
                            Ok(crate::layout::RuleTokens::Setter(
                                quote::quote! { $method(#value) }
                            ))
                        }
                    ),*
                }
            }
        }
    };
}

pub(crate) use properties;
