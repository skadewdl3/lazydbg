use proc_macro2::TokenStream as TokenStream2;

use crate::layout::{RuleTokens, RustExpr, TrackList, deserialize_css, properties};

pub fn rule(property: &str, value: TokenStream2) -> syn::Result<Option<RuleTokens>> {
    deserialize_css::<Property>(property)
        .map(|property| property.emit(value).map(Some))
        .unwrap_or(Ok(None))
}

properties! {
    Property {
        Columns => ("columns", columns, TrackList),
        Rows => ("rows", rows, TrackList),
        Column => ("column", column, RustExpr),
        Row => ("row", row, RustExpr),
        ColumnSpan => ("column-span", column_span, RustExpr),
        RowSpan => ("row-span", row_span, RustExpr),
        ColumnEnd => ("column-end", column_end, RustExpr),
        RowEnd => ("row-end", row_end, RustExpr),
    }
}
