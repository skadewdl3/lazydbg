//! Implementation of the `style!` macro.
//!
//! CSS-flavored, semicolon-terminated grammar. Property names may be
//! written with hyphens (`flex-basis`) or underscores (`flex_basis`) —
//! both spellings work identically:
//!
//! ```ignore
//! style! {
//!     background: green;
//!     color: rgb(255, 23, 45);
//!     flex-basis: 2;      // or `flex-basis: auto;`
//!     flex-grow: 1;
//!     bold;
//!
//!     // escape hatch: anything not covered above, applied as a closure
//!     // over the accumulated value. Never blocked on this file being
//!     // updated to know about a new ratatui/reactatui builder method.
//!     --color: |s| s.underline_color(Color::Blue);
//!     --layout: |s| s.row_span(2);
//! }
//! ```
//!
//! Recognized color values: a named color (`red`, `dark-gray`,
//! `light-blue`, ...), `rgb(r, g, b)`, or any `Color`-typed expression
//! (e.g. `Color::Indexed(3)` or a variable).
//!
//! Recognized `flex-basis` values: `auto`, or any cell-count expression
//! — wrapped in `FlexBasis::Length` automatically.
//!
//! Every other property (`flex-grow`, `flex-shrink`, `column`, `row`,
//! `justify-content`, `align-items`, ...) takes a plain Rust expression,
//! same as before.

use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{format_ident, quote};
use syn::{
    Expr, Ident, Token,
    parse::{Parse, ParseStream, discouraged::Speculative},
    punctuated::Punctuated,
    spanned::Spanned,
};

enum StyleValueKind {
    Expr(Expr),
    FlexBasisAuto,
    NamedColor(&'static str),
    Rgb(Expr, Expr, Expr),
    If {
        condition: Expr,
        then_val: Box<StyleValueKind>,
        else_val: Option<Box<StyleValueKind>>,
    },
}

enum StyleEntry {
    Flag(String, Span),
    KeyValue {
        key: String,
        key_span: Span,
        value: StyleValueKind,
    },
    EscapeColor(Expr),
    EscapeLayout(Expr),
}

/// Reads one `word` or `hyphen-joined-word` sequence (e.g. `flex`,
/// `dark-gray`, `flex-basis`) starting at the current position.
fn read_kebab_word(input: ParseStream) -> syn::Result<(String, Span)> {
    let first: Ident = input.parse()?;
    let span = first.span();
    let mut word = first.to_string();
    while input.peek(Token![-]) && input.peek2(Ident) {
        input.parse::<Token![-]>()?;
        let seg: Ident = input.parse()?;
        word.push('-');
        word.push_str(&seg.to_string());
    }
    Ok((word, span))
}

fn parse_flex_basis_value(input: ParseStream) -> syn::Result<StyleValueKind> {
    if input.peek(Token![if]) {
        return parse_if_value(input, parse_flex_basis_value);
    }
    let fork = input.fork();
    if let Ok((word, _)) = read_kebab_word(&fork)
        && word == "auto"
    {
        input.advance_to(&fork);
        return Ok(StyleValueKind::FlexBasisAuto);
    }

    let expr: Expr = input.parse().map_err(|e| {
        syn::Error::new(
            e.span(),
            format!(
                "expected `auto` or a cell-count expression for `flex-basis` \
                 (e.g. `flex-basis: 3;` or `flex-basis: auto;`) — {e}"
            ),
        )
    })?;
    Ok(StyleValueKind::Expr(expr))
}

fn parse_if_value<R>(input: ParseStream, recurse: R) -> syn::Result<StyleValueKind>
where
    R: Fn(ParseStream) -> syn::Result<StyleValueKind> + Copy,
{
    input.parse::<Token![if]>()?;
    let condition: Expr = Expr::parse_without_eager_brace(input)?;

    let then_buf;
    syn::braced!(then_buf in input);
    let then_val = recurse(&then_buf)?;

    // Check if there's an `else`
    let else_val = if input.peek(Token![else]) {
        input.parse::<Token![else]>()?;
        if input.peek(Token![if]) {
            Some(Box::new(parse_if_value(input, recurse)?))
        } else {
            let else_buf;
            syn::braced!(else_buf in input);
            Some(Box::new(recurse(&else_buf)?))
        }
    } else {
        None
    };

    Ok(StyleValueKind::If {
        condition,
        then_val: Box::new(then_val),
        else_val,
    })
}

fn named_color(word: &str) -> Option<&'static str> {
    Some(match word {
        "reset" => "Reset",
        "black" => "Black",
        "red" => "Red",
        "green" => "Green",
        "yellow" => "Yellow",
        "blue" => "Blue",
        "magenta" => "Magenta",
        "cyan" => "Cyan",
        "white" => "White",
        "gray" | "grey" => "Gray",
        "dark-gray" | "dark-grey" | "darkgray" | "darkgrey" => "DarkGray",
        "light-red" | "lightred" => "LightRed",
        "light-green" | "lightgreen" => "LightGreen",
        "light-yellow" | "lightyellow" => "LightYellow",
        "light-blue" | "lightblue" => "LightBlue",
        "light-magenta" | "lightmagenta" => "LightMagenta",
        "light-cyan" | "lightcyan" => "LightCyan",
        _ => return None,
    })
}

/// Returns true if an `if` chain ends without an `else` block.
fn is_elseless_if(value: &StyleValueKind) -> bool {
    match value {
        StyleValueKind::If { else_val: None, .. } => true,
        StyleValueKind::If {
            else_val: Some(boxed),
            ..
        } => is_elseless_if(boxed),
        _ => false,
    }
}

/// Generates a statement that conditionally applies a color method.
fn emit_color_stmt(method: &Ident, value: StyleValueKind) -> syn::Result<TokenStream2> {
    match value {
        StyleValueKind::If {
            condition,
            then_val,
            else_val: Some(else_val),
        } => {
            let then_color = emit_color(*then_val)?;
            // Recursively handle `else if`
            let else_stmt = emit_color_stmt(method, *else_val)?;
            Ok(quote! {
                if #condition {
                    __style_color = __style_color.#method(#then_color);
                } else #else_stmt
            })
        }
        StyleValueKind::If {
            condition,
            then_val,
            else_val: None,
        } => {
            let then_color = emit_color(*then_val)?;
            Ok(quote! {
                if #condition {
                    __style_color = __style_color.#method(#then_color);
                }
            })
        }
        _ => Err(syn::Error::new(
            Span::call_site(),
            "expected an `if` expression",
        )),
    }
}

/// Generates a statement that conditionally applies a layout method.
fn emit_layout_stmt(method_name: &str, value: StyleValueKind) -> syn::Result<TokenStream2> {
    let method = format_ident!("{method_name}");
    match value {
        StyleValueKind::If {
            condition,
            then_val,
            else_val: Some(else_val),
        } => {
            let then_layout = emit_layout(method_name, *then_val)?;
            let else_stmt = emit_layout_stmt(method_name, *else_val)?;
            Ok(quote! {
                if #condition {
                    __style_layout = __style_layout.#method(#then_layout);
                } else #else_stmt
            })
        }
        StyleValueKind::If {
            condition,
            then_val,
            else_val: None,
        } => {
            let then_layout = emit_layout(method_name, *then_val)?;
            Ok(quote! {
                if #condition {
                    __style_layout = __style_layout.#method(#then_layout);
                }
            })
        }
        _ => Err(syn::Error::new(
            Span::call_site(),
            "expected an `if` expression",
        )),
    }
}

fn emit_color(value: StyleValueKind) -> syn::Result<TokenStream2> {
    Ok(match value {
        StyleValueKind::NamedColor(variant) => {
            let variant = format_ident!("{variant}");
            quote! { ::ratatui::style::Color::#variant }
        }
        StyleValueKind::Rgb(r, g, b) => {
            quote! { ::ratatui::style::Color::Rgb((#r) as u8, (#g) as u8, (#b) as u8) }
        }
        StyleValueKind::Expr(expr) => {
            quote! { #expr }
        }
        StyleValueKind::If {
            condition,
            then_val,
            else_val: Some(else_val), // <-- Match Some here
        } => {
            let then = emit_color(*then_val)?;
            let else_ = emit_color(*else_val)?;

            quote! {
                if #condition { #then } else { #else_ }
            }
        }
        // <-- Add fallback error arm
        StyleValueKind::If { condition, .. } => {
            return Err(syn::Error::new(
                condition.span(),
                "an `if` without an `else` can only be used as a top-level property value",
            ));
        }
        StyleValueKind::FlexBasisAuto => {
            return Err(syn::Error::new(
                Span::call_site(),
                "`auto` isn't a valid color value",
            ));
        }
    })
}

fn emit_layout(method: &str, value: StyleValueKind) -> syn::Result<TokenStream2> {
    Ok(match value {
        StyleValueKind::If {
            condition,
            then_val,
            else_val: Some(else_val), // <-- Match Some here
        } => {
            let then_layout = emit_layout(method, *then_val)?;
            let else_layout = emit_layout(method, *else_val)?;

            quote! {
                if #condition { #then_layout } else { #else_layout }
            }
        }
        // <-- Add fallback error arm
        StyleValueKind::If { condition, .. } => {
            return Err(syn::Error::new(
                condition.span(),
                "an `if` without an `else` can only be used as a top-level property value",
            ));
        }
        StyleValueKind::FlexBasisAuto if method == "flex_basis" => {
            quote! { ::reactatui::layout::FlexBasis::Auto }
        }
        StyleValueKind::Expr(expr) if method == "flex_basis" => {
            quote! { ::reactatui::layout::FlexBasis::Length((#expr) as u16) }
        }
        StyleValueKind::Expr(expr) if matches!(method, "flex_grow" | "flex_shrink") => {
            quote! { (#expr) as f32 }
        }
        StyleValueKind::Expr(expr)
            if matches!(method, "column" | "row" | "column_span" | "row_span") =>
        {
            quote! { (#expr) as usize }
        }
        StyleValueKind::Expr(expr) => {
            quote! { #expr }
        }
        StyleValueKind::FlexBasisAuto => {
            return Err(syn::Error::new(
                Span::call_site(),
                "`auto` is only valid for `flex-basis`",
            ));
        }
        StyleValueKind::NamedColor(_) | StyleValueKind::Rgb(..) => {
            return Err(syn::Error::new(
                Span::call_site(),
                "expected a layout value",
            ));
        }
    })
}

fn parse_color_value(input: ParseStream) -> syn::Result<StyleValueKind> {
    if input.peek(Token![if]) {
        return parse_if_value(input, parse_color_value);
    }

    let fork = input.fork();
    if let Ok((word, span)) = read_kebab_word(&fork) {
        if word == "rgb" && fork.peek(syn::token::Paren) {
            let content;
            syn::parenthesized!(content in fork);
            let args = Punctuated::<Expr, Token![,]>::parse_terminated(&content)?;
            if args.len() != 3 {
                return Err(syn::Error::new(
                    span,
                    format!(
                        "`rgb(...)` needs exactly 3 arguments (red, green, blue), found {}",
                        args.len()
                    ),
                ));
            }
            input.advance_to(&fork);
            let mut it = args.into_iter();
            let r = it.next().unwrap();
            let g = it.next().unwrap();
            let b = it.next().unwrap();
            return Ok(StyleValueKind::Rgb(r, g, b));
        }
        if let Some(variant) = named_color(&word.to_ascii_lowercase()) {
            input.advance_to(&fork);
            return Ok(StyleValueKind::NamedColor(variant));
        }
    }

    let expr: Expr = input.parse().map_err(|e| {
        syn::Error::new(
            e.span(),
            format!(
                "expected a color here — try a named color (`green`, `dark-gray`, ...), \
                 `rgb(r, g, b)`, or a `Color` expression — {e}"
            ),
        )
    })?;
    Ok(StyleValueKind::Expr(expr))
}

impl Parse for StyleEntry {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        // `--color: |s| ...` / `--layout: |s| ...` escape hatch.
        if input.peek(Token![-]) && input.peek2(Token![-]) {
            input.parse::<Token![-]>()?;
            input.parse::<Token![-]>()?;
            let ident: Ident = input.parse().map_err(|_| {
                syn::Error::new(input.span(), "expected `color` or `layout` after `--`")
            })?;
            input.parse::<Token![:]>().map_err(|_| {
                syn::Error::new(ident.span(), format!("expected `:` after `--{ident}`"))
            })?;
            let expr: Expr = input.parse()?;
            return match ident.to_string().as_str() {
                "color" => Ok(StyleEntry::EscapeColor(expr)),
                "layout" => Ok(StyleEntry::EscapeLayout(expr)),
                other => Err(syn::Error::new(
                    ident.span(),
                    format!(
                        "unknown escape hatch `--{other}` — only `--color: |s| ...` and \
                         `--layout: |s| ...` are supported"
                    ),
                )),
            };
        }
        if input.peek(Token![-]) {
            return Err(syn::Error::new(
                input.span(),
                "unexpected `-` — style properties don't start with a minus sign; did you \
                 mean an escape hatch like `--color: |s| ...`?",
            ));
        }

        let (raw_key, key_span) = read_kebab_word(input).map_err(|_| {
            syn::Error::new(
                input.span(),
                "expected a style property name here (e.g. `flex-grow`, `color`, `bold`)",
            )
        })?;
        let key = raw_key.replace('-', "_");

        if !input.peek(Token![:]) {
            return Ok(StyleEntry::Flag(key, key_span));
        }
        input.parse::<Token![:]>().map_err(|_| {
            syn::Error::new(
                key_span,
                format!(
                    "expected `:` after `{raw_key}` (or did you mean a bare flag like `bold;`?)"
                ),
            )
        })?;

        let value = match key.as_str() {
            "flex_basis" => parse_flex_basis_value(input)?,
            "color" | "fg" | "background" | "bg" | "underline_color" => parse_color_value(input)?,
            _ => StyleValueKind::Expr(input.parse().map_err(|e| {
                syn::Error::new(
                    e.span(),
                    format!("expected a value expression for `{raw_key}` — {e}"),
                )
            })?),
        };

        Ok(StyleEntry::KeyValue {
            key,
            key_span,
            value,
        })
    }
}

struct StyleInput {
    entries: Punctuated<StyleEntry, Token![;]>,
}

impl Parse for StyleInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(StyleInput {
            entries: Punctuated::parse_terminated(input)?,
        })
    }
}

#[derive(Clone, Copy)]
enum Target {
    Color,
    Layout,
}

/// Shorthand key -> (which half of `CombinedStyle`, builder method name).
/// Keys here are canonical snake_case; hyphenated user input is normalized
/// to this form before lookup.
fn lookup_key(name: &str) -> Option<(Target, &'static str)> {
    use Target::*;
    Some(match name {
        "fg" | "color" => (Color, "fg"),
        "bg" | "background" => (Color, "bg"),
        "underline_color" => (Color, "underline_color"),
        "add_modifier" => (Color, "add_modifier"),
        "remove_modifier" => (Color, "remove_modifier"),

        "justify_content" => (Layout, "justify_content"),
        "align_content" => (Layout, "align_content"),
        "align_items" => (Layout, "align_items"),
        "justify_items" => (Layout, "justify_items"),
        "align_self" => (Layout, "align_self"),
        "justify_self" => (Layout, "justify_self"),
        "flex_grow" => (Layout, "flex_grow"),
        "flex_shrink" => (Layout, "flex_shrink"),
        "flex_basis" => (Layout, "flex_basis"),
        "gap" => (Layout, "gap"),
        "column" => (Layout, "column"),
        "row" => (Layout, "row"),
        "column_span" => (Layout, "column_span"),
        "row_span" => (Layout, "row_span"),

        _ => return None,
    })
}

/// Display form (kebab-case) of every known property, used only for the
/// "did you mean" suggestion and the full list in error messages.
const ALL_KEYS: &[&str] = &[
    "fg",
    "color",
    "bg",
    "background",
    "underline-color",
    "add-modifier",
    "remove-modifier",
    "justify-content",
    "align-content",
    "align-items",
    "justify-items",
    "align-self",
    "justify-self",
    "flex-grow",
    "flex-shrink",
    "flex-basis",
    "column",
    "row",
    "column-span",
    "row-span",
];

/// Bare-word flags, e.g. `bold` instead of `add_modifier: Modifier::BOLD`.
fn lookup_flag(name: &str) -> Option<(Target, TokenStream2)> {
    use Target::*;
    Some(match name {
        "bold" => (
            Color,
            quote! { add_modifier(::ratatui::style::Modifier::BOLD) },
        ),
        "dim" => (
            Color,
            quote! { add_modifier(::ratatui::style::Modifier::DIM) },
        ),
        "italic" => (
            Color,
            quote! { add_modifier(::ratatui::style::Modifier::ITALIC) },
        ),
        "underlined" => (
            Color,
            quote! { add_modifier(::ratatui::style::Modifier::UNDERLINED) },
        ),
        "slow_blink" => (
            Color,
            quote! { add_modifier(::ratatui::style::Modifier::SLOW_BLINK) },
        ),
        "rapid_blink" => (
            Color,
            quote! { add_modifier(::ratatui::style::Modifier::RAPID_BLINK) },
        ),
        "reversed" => (
            Color,
            quote! { add_modifier(::ratatui::style::Modifier::REVERSED) },
        ),
        "hidden" => (
            Color,
            quote! { add_modifier(::ratatui::style::Modifier::HIDDEN) },
        ),
        "crossed_out" => (
            Color,
            quote! { add_modifier(::ratatui::style::Modifier::CROSSED_OUT) },
        ),
        _ => return None,
    })
}

const ALL_FLAGS: &[&str] = &[
    "bold",
    "dim",
    "italic",
    "underlined",
    "slow_blink",
    "rapid_blink",
    "reversed",
    "hidden",
    "crossed_out",
];

fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut dp = vec![vec![0usize; b.len() + 1]; a.len() + 1];
    for (i, row) in dp.iter_mut().enumerate() {
        row[0] = i;
    }
    for (j, cell) in dp[0].iter_mut().enumerate() {
        *cell = j;
    }
    for i in 1..=a.len() {
        for j in 1..=b.len() {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            dp[i][j] = (dp[i - 1][j] + 1)
                .min(dp[i][j - 1] + 1)
                .min(dp[i - 1][j - 1] + cost);
        }
    }
    dp[a.len()][b.len()]
}

fn closest_match<'a>(name: &str, candidates: &[&'a str]) -> Option<&'a str> {
    candidates
        .iter()
        .map(|c| (*c, levenshtein(&name.replace('_', "-"), c)))
        .filter(|(_, d)| *d <= 2)
        .min_by_key(|(_, d)| *d)
        .map(|(c, _)| c)
}

pub fn expand(input: TokenStream2) -> TokenStream2 {
    let parsed: StyleInput = match syn::parse2(input) {
        Ok(p) => p,
        Err(err) => return err.to_compile_error(),
    };

    let mut stmts = Vec::new();

    for entry in parsed.entries {
        match entry {
            StyleEntry::EscapeColor(expr) => {
                stmts.push(quote! { __style_color = (#expr)(__style_color); });
            }
            StyleEntry::EscapeLayout(expr) => {
                stmts.push(quote! { __style_layout = (#expr)(__style_layout); });
            }
            StyleEntry::Flag(name, span) => match lookup_flag(&name) {
                Some((Target::Color, call)) => {
                    stmts.push(quote! {
                        __style_color = __style_color.#call;
                    });
                }

                Some((Target::Layout, call)) => {
                    stmts.push(quote! {
                        __style_layout = __style_layout.#call;
                    });
                }

                None => {
                    // existing error handling
                }
            },
            StyleEntry::KeyValue {
                key,
                key_span,
                value,
            } => match lookup_key(&key) {
                Some((Target::Color, method)) => {
                    // Intercept incomplete `if`/`else if` chains
                    if is_elseless_if(&value) {
                        let method_ident = format_ident!("{method}");
                        match emit_color_stmt(&method_ident, value) {
                            Ok(ts) => stmts.push(ts),
                            Err(err) => return err.to_compile_error(),
                        }
                    } else {
                        let color_expr = match emit_color(value) {
                            Ok(ts) => ts,
                            Err(err) => return err.to_compile_error(),
                        };
                        let method_ident = format_ident!("{method}");
                        stmts.push(quote! {
                            __style_color = __style_color.#method_ident(#color_expr);
                        });
                    }
                }

                Some((Target::Layout, method)) => {
                    // Intercept incomplete `if`/`else if` chains
                    if is_elseless_if(&value) {
                        match emit_layout_stmt(method, value) {
                            Ok(ts) => stmts.push(ts),
                            Err(err) => return err.to_compile_error(),
                        }
                    } else {
                        let layout_expr = match emit_layout(method, value) {
                            Ok(ts) => ts,
                            Err(err) => return err.to_compile_error(),
                        };
                        let m = format_ident!("{method}");
                        stmts.push(quote! {
                            __style_layout = __style_layout.#m(#layout_expr);
                        });
                    }
                }

                None => {
                    let hint = match closest_match(&key, ALL_KEYS) {
                        Some(s) => format!(" — did you mean `{s}`?"),
                        None => String::new(),
                    };

                    let msg = format!(
                        "`{key}` isn't a recognized style property{hint}. Known properties: {}. \
                         For anything else, use `--color: |s| s.your_method(..)` or \
                         `--layout: |s| s.your_method(..)`.",
                        ALL_KEYS.join(", ")
                    );

                    return syn::Error::new(key_span, msg).to_compile_error();
                }
            },
        }
    }

    quote! {{
        let mut __style_color = ::ratatui::style::Style::default();
        let mut __style_layout = ::reactatui::layout::Style::default();
        #(#stmts)*
        ::reactatui::style::CombinedStyle {
            color: __style_color,
            layout: __style_layout,
        }
    }}
}
