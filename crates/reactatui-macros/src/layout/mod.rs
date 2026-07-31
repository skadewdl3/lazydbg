mod flex;
mod grid;

use proc_macro2::{Delimiter, Ident, TokenStream as TokenStream2, TokenTree};
use quote::{format_ident, quote};
use serde::Deserialize;
use serde::de::value::{BorrowedStrDeserializer, Error as SerdeValueError};
use syn::parse::Parser as _;

pub fn layout(input: TokenStream2) -> TokenStream2 {
    match Parser::new(input, Target::Layout).parse() {
        Ok(styles) => styles,
        Err(error) => error.to_compile_error(),
    }
}

#[derive(Clone, Copy)]
pub(crate) enum Target {
    Layout,
    Style,
}

#[derive(Debug)]
pub(crate) enum RuleTokens {
    Setter(TokenStream2),
    Replace(TokenStream2),
    Raw(TokenStream2),
}

pub(crate) trait CssValue {
    fn parse_tokens(value: TokenStream2, property: &'static str) -> syn::Result<TokenStream2>;
}

pub(crate) struct RustExpr;

impl CssValue for RustExpr {
    fn parse_tokens(value: TokenStream2, _: &'static str) -> syn::Result<TokenStream2> {
        Ok(value)
    }
}

pub(crate) struct TrackList;

impl CssValue for TrackList {
    fn parse_tokens(value: TokenStream2, property: &'static str) -> syn::Result<TokenStream2> {
        let tokens: Vec<_> = value.clone().into_iter().collect();
        if is_css_track_list(&tokens) {
            let tracks = value.to_string();
            if tracks
                .split(|ch: char| ch.is_ascii_whitespace() || ch == ',')
                .filter(|track| !track.is_empty())
                .all(|track| deserialize_css::<flex::Size>(track).is_some())
            {
                let sizes = tracks
                    .split(|ch: char| ch.is_ascii_whitespace() || ch == ',')
                    .filter(|track| !track.is_empty())
                    .map(
                        |track| match deserialize_css::<flex::Size>(track).expect("validated") {
                            flex::Size::Auto => quote! { ::reactatui::layout::Size::Auto },
                            flex::Size::Length(value) => {
                                quote! { ::reactatui::layout::Size::Length(#value) }
                            }
                            flex::Size::Fr(value) => {
                                quote! { ::reactatui::layout::Size::Fr(#value) }
                            }
                            flex::Size::Percent(value) => {
                                quote! { ::reactatui::layout::Size::Percent(#value) }
                            }
                        },
                    )
                    .collect::<Vec<_>>();
                return Ok(quote! { vec![#(#sizes),*] });
            }

            return Err(invalid_layout_value(
                &value,
                property,
                "a space- or comma-separated list of `auto`, integers, `<number>fr`, or `<number>%` tracks",
            ));
        }

        Ok(value)
    }
}

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

struct Parser {
    tokens: Vec<TokenTree>,
    pos: usize,
    target: Target,
}

#[derive(Clone)]
struct CssName {
    parts: Vec<Ident>,
}

impl Parser {
    fn new(tokens: TokenStream2, target: Target) -> Self {
        Self {
            tokens: tokens.into_iter().collect(),
            pos: 0,
            target,
        }
    }

    fn parse(&mut self) -> syn::Result<TokenStream2> {
        let style = match self.target {
            Target::Layout => format_ident!("__reactatui_layout_style"),
            Target::Style => format_ident!("__reactatui_style"),
        };
        let body = self.parse_style_block(&style)?;
        let initial = match self.target {
            Target::Layout => quote! { ::reactatui::layout::Style::default() },
            Target::Style => quote! { ::ratatui::style::Style::new() },
        };
        Ok(quote! {{
            let mut #style = #initial;
            #body
            #style
        }})
    }

    fn parse_style_block(&mut self, style: &Ident) -> syn::Result<TokenStream2> {
        let mut out = Vec::new();
        while !self.is_done() {
            self.skip_attrs();
            if self.is_done() {
                break;
            }

            if self.peek_ident("if") {
                out.push(self.parse_block_if(style)?);
            } else if self.peek_ident("match") {
                out.push(self.parse_block_match(style)?);
            } else {
                out.push(self.parse_rule(style)?);
            }
        }
        Ok(quote! { #(#out)* })
    }

    fn parse_rule(&mut self, style: &Ident) -> syn::Result<TokenStream2> {
        let name = self.parse_css_name()?;
        self.expect_punct(':')?;
        let value = self.collect_rule_value();
        if value.is_empty() {
            return Err(self.error("layout rule requires a value"));
        }
        self.consume_punct(';');
        self.gen_rule(style, &name, value)
    }

    fn parse_block_if(&mut self, style: &Ident) -> syn::Result<TokenStream2> {
        self.expect_keyword("if")?;
        let (condition, body) = self.collect_until_brace_group()?;
        let then_body = Parser::new(body, self.target).parse_style_block(style)?;

        let else_body = if self.peek_ident("else") {
            self.expect_keyword("else")?;
            if self.peek_ident("if") {
                Some(self.parse_block_if(style)?)
            } else {
                Some(
                    Parser::new(self.expect_brace_group()?, self.target)
                        .parse_style_block(style)?,
                )
            }
        } else {
            None
        };

        Ok(match else_body {
            Some(else_body) => quote! {
                if #condition {
                    #then_body
                } else {
                    #else_body
                }
            },
            None => quote! {
                if #condition {
                    #then_body
                }
            },
        })
    }

    fn parse_block_match(&mut self, style: &Ident) -> syn::Result<TokenStream2> {
        self.expect_keyword("match")?;
        let (scrutinee, body) = self.collect_until_brace_group()?;
        let mut inner = Parser::new(body, self.target);
        let mut arms = Vec::new();
        let mut shorthand_property = None;

        while !inner.is_done() {
            inner.skip_attrs();
            if inner.is_done() {
                break;
            }
            arms.push(inner.parse_match_arm(style, &mut shorthand_property)?);
            inner.consume_punct(',');
        }

        Ok(quote! {
            match #scrutinee {
                #(#arms),*
            }
        })
    }

    fn parse_match_arm(
        &mut self,
        style: &Ident,
        shorthand_property: &mut Option<CssName>,
    ) -> syn::Result<TokenStream2> {
        let mut pattern_tokens = TokenStream2::new();
        while !self.is_done() {
            if self.starts_fat_arrow() || self.peek_ident("if") {
                break;
            }
            pattern_tokens.extend([self.tokens[self.pos].clone()]);
            self.pos += 1;
        }
        if pattern_tokens.is_empty() {
            return Err(self.error("match arm requires a pattern"));
        }
        let pattern = syn::Pat::parse_multi_with_leading_vert.parse2(pattern_tokens)?;

        let guard = if self.peek_ident("if") {
            self.pos += 1;
            let mut guard = TokenStream2::new();
            while !self.is_done() && !self.starts_fat_arrow() {
                guard.extend([self.tokens[self.pos].clone()]);
                self.pos += 1;
            }
            if guard.is_empty() {
                return Err(self.error("match guard requires an expression"));
            }
            Some(guard)
        } else {
            None
        };

        self.expect_fat_arrow()?;
        self.skip_attrs();

        let body = if let Some(TokenTree::Group(group)) = self.peek().cloned()
            && group.delimiter() == Delimiter::Brace
        {
            self.pos += 1;
            Parser::new(group.stream(), self.target).parse_style_block(style)?
        } else if self.next_is_rule() {
            let name = self.peek_css_name()?;
            *shorthand_property = Some(name);
            self.parse_rule(style)?
        } else {
            let Some(name) = shorthand_property.clone() else {
                return Err(
                    self.error("match arm shorthand values require an earlier property arm")
                );
            };
            let value = self.collect_match_value();
            if value.is_empty() {
                return Err(self.error("match arm shorthand requires a value"));
            }
            self.consume_punct(';');
            self.gen_rule(style, &name, value)?
        };

        let guard = guard.as_ref().map(|guard| quote! { if #guard });
        Ok(quote! {
            #pattern #guard => {
                #body
            }
        })
    }

    fn gen_rule(
        &self,
        style: &Ident,
        name: &CssName,
        value: TokenStream2,
    ) -> syn::Result<TokenStream2> {
        let rule = property_value(self.target, name, value)?;
        Ok(match rule {
            RuleTokens::Setter(setter) => quote! {
                #style = #style.#setter;
            },
            RuleTokens::Replace(value) => quote! {
                #style = #value;
            },
            RuleTokens::Raw(statements) => statements,
        })
    }

    fn parse_css_name(&mut self) -> syn::Result<CssName> {
        let name = self.peek_css_name()?;
        self.pos += name.parts.len().saturating_mul(2).saturating_sub(1);
        Ok(name)
    }

    fn peek_css_name(&self) -> syn::Result<CssName> {
        let mut parts = Vec::new();
        let mut pos = self.pos;
        match self.tokens.get(pos).cloned() {
            Some(TokenTree::Ident(ident)) => {
                parts.push(ident);
                pos += 1;
            }
            _ => return Err(self.error("expected layout property name")),
        }

        while matches!(self.tokens.get(pos), Some(TokenTree::Punct(punct)) if punct.as_char() == '-')
            && matches!(self.tokens.get(pos + 1), Some(TokenTree::Ident(_)))
        {
            if let Some(TokenTree::Ident(ident)) = self.tokens.get(pos + 1).cloned() {
                parts.push(ident);
            }
            pos += 2;
        }

        Ok(CssName { parts })
    }

    fn next_is_rule(&self) -> bool {
        let Ok(name) = self.peek_css_name() else {
            return false;
        };
        let pos = self.pos + name.parts.len().saturating_mul(2).saturating_sub(1);
        matches!(self.tokens.get(pos), Some(TokenTree::Punct(punct)) if punct.as_char() == ':')
    }

    fn collect_rule_value(&mut self) -> TokenStream2 {
        let mut out = TokenStream2::new();
        while !self.is_done() && !self.peek_punct(';') {
            out.extend([self.tokens[self.pos].clone()]);
            self.pos += 1;
        }
        out
    }

    fn collect_match_value(&mut self) -> TokenStream2 {
        let mut out = TokenStream2::new();
        while !self.is_done() && !self.peek_punct(';') && !self.peek_punct(',') {
            out.extend([self.tokens[self.pos].clone()]);
            self.pos += 1;
        }
        out
    }

    fn collect_until_brace_group(&mut self) -> syn::Result<(TokenStream2, TokenStream2)> {
        let mut head = TokenStream2::new();
        while !self.is_done() {
            match self.peek().cloned() {
                Some(TokenTree::Group(group)) if group.delimiter() == Delimiter::Brace => {
                    self.pos += 1;
                    return Ok((head, group.stream()));
                }
                Some(token) => {
                    head.extend([token]);
                    self.pos += 1;
                }
                None => break,
            }
        }
        Err(self.error("expected a braced block"))
    }

    fn expect_brace_group(&mut self) -> syn::Result<TokenStream2> {
        match self.peek().cloned() {
            Some(TokenTree::Group(group)) if group.delimiter() == Delimiter::Brace => {
                self.pos += 1;
                Ok(group.stream())
            }
            _ => Err(self.error("expected a braced block")),
        }
    }

    fn skip_attrs(&mut self) {
        loop {
            let hash =
                matches!(self.peek(), Some(TokenTree::Punct(punct)) if punct.as_char() == '#');
            let bang =
                matches!(self.peek_n(1), Some(TokenTree::Punct(punct)) if punct.as_char() == '!');
            let group_offset = if hash && bang { 2 } else { 1 };
            let bracket = matches!(
                self.peek_n(group_offset),
                Some(TokenTree::Group(group)) if group.delimiter() == Delimiter::Bracket
            );
            if hash && bracket {
                self.pos += group_offset + 1;
            } else {
                break;
            }
        }
    }

    fn expect_keyword(&mut self, keyword: &str) -> syn::Result<()> {
        if self.peek_ident(keyword) {
            self.pos += 1;
            Ok(())
        } else {
            Err(self.error(format!("expected `{keyword}`")))
        }
    }

    fn expect_punct(&mut self, ch: char) -> syn::Result<()> {
        if self.consume_punct(ch) {
            Ok(())
        } else {
            Err(self.error(format!("expected `{ch}`")))
        }
    }

    fn consume_punct(&mut self, ch: char) -> bool {
        if self.peek_punct(ch) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn starts_fat_arrow(&self) -> bool {
        matches!(self.peek(), Some(TokenTree::Punct(punct)) if punct.as_char() == '=')
            && matches!(self.peek_n(1), Some(TokenTree::Punct(punct)) if punct.as_char() == '>')
    }

    fn expect_fat_arrow(&mut self) -> syn::Result<()> {
        if self.starts_fat_arrow() {
            self.pos += 2;
            Ok(())
        } else {
            Err(self.error("expected `=>`"))
        }
    }

    fn peek_ident(&self, name: &str) -> bool {
        matches!(self.peek(), Some(TokenTree::Ident(ident)) if ident == name)
    }

    fn peek_punct(&self, ch: char) -> bool {
        matches!(self.peek(), Some(TokenTree::Punct(punct)) if punct.as_char() == ch)
    }

    fn peek(&self) -> Option<&TokenTree> {
        self.tokens.get(self.pos)
    }

    fn peek_n(&self, offset: usize) -> Option<&TokenTree> {
        self.tokens.get(self.pos + offset)
    }

    fn is_done(&self) -> bool {
        self.pos >= self.tokens.len()
    }

    fn error(&self, message: impl std::fmt::Display) -> syn::Error {
        let span = self
            .peek()
            .map(TokenTree::span)
            .unwrap_or_else(proc_macro2::Span::call_site);
        syn::Error::new(span, message)
    }
}

impl CssName {
    fn as_kebab(&self) -> String {
        self.parts
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("-")
    }
}

fn property_value(target: Target, name: &CssName, value: TokenStream2) -> syn::Result<RuleTokens> {
    let property = name.as_kebab();
    match target {
        Target::Layout => {
            if let Some(rule) = flex::rule(&property, value.clone())? {
                return Ok(rule);
            }
            if let Some(rule) = grid::rule(&property, value)? {
                return Ok(rule);
            }
        }
        Target::Style => return crate::style::property_value(&property, value),
    }

    Err(syn::Error::new_spanned(
        name.parts.first().expect("css names are non-empty"),
        format!("unknown {} property `{property}`", target.name()),
    ))
}

impl Target {
    fn name(self) -> &'static str {
        match self {
            Self::Layout => "layout",
            Self::Style => "style",
        }
    }
}

pub(crate) fn parse_style(input: TokenStream2) -> syn::Result<TokenStream2> {
    Parser::new(input, Target::Style).parse()
}

pub(crate) fn enum_value<T>(
    value: TokenStream2,
    mapper: fn(T) -> TokenStream2,
    property: &'static str,
    expected: &'static str,
) -> syn::Result<TokenStream2>
where
    T: for<'de> Deserialize<'de>,
{
    parse_css_value(
        value,
        move |name| deserialize_css::<T>(name).map(mapper),
        |value| invalid_layout_value(value, property, expected),
    )
}

pub(crate) fn parse_css_value(
    value: TokenStream2,
    mapper: impl Fn(&str) -> Option<TokenStream2> + Copy,
    invalid: impl Fn(&TokenStream2) -> syn::Error + Copy,
) -> syn::Result<TokenStream2> {
    let tokens: Vec<_> = value.into_iter().collect();
    if starts_ident(&tokens, "if") {
        return parse_inline_if(tokens, mapper, invalid);
    }

    let name = css_value_from_tokens(&tokens);
    if let Some(name) = name
        && let Some(mapped) = mapper(&name)
    {
        return Ok(mapped);
    }

    if is_css_value(&tokens) {
        return Err(invalid(&tokens.into_iter().collect()));
    }

    Ok(tokens.into_iter().collect())
}

fn parse_inline_if(
    tokens: Vec<TokenTree>,
    mapper: impl Fn(&str) -> Option<TokenStream2> + Copy,
    invalid: impl Fn(&TokenStream2) -> syn::Error + Copy,
) -> syn::Result<TokenStream2> {
    let mut parser = Parser {
        tokens,
        pos: 0,
        target: Target::Layout,
    };
    parser.expect_keyword("if")?;
    let (condition, body) = parser.collect_until_brace_group()?;
    let then_value = parse_css_value(body, mapper, invalid)?;

    let else_value = if parser.peek_ident("else") {
        parser.expect_keyword("else")?;
        if parser.peek_ident("if") {
            let nested = parse_inline_if(parser.tokens[parser.pos..].to_vec(), mapper, invalid)?;
            parser.pos = parser.tokens.len();
            Some(nested)
        } else {
            Some(parse_css_value(
                parser.expect_brace_group()?,
                mapper,
                invalid,
            )?)
        }
    } else {
        None
    };

    if !parser.is_done() {
        return Err(parser.error("unexpected tokens after inline if value"));
    }

    Ok(match else_value {
        Some(else_value) => quote! {
            if #condition {
                #then_value
            } else {
                #else_value
            }
        },
        None => quote! {
            if #condition {
                #then_value
            }
        },
    })
}

fn invalid_layout_value(value: &TokenStream2, property: &str, expected: &str) -> syn::Error {
    let display_value = value.to_string();
    syn::Error::new_spanned(
        value,
        format!(
            "invalid value `{display_value}` for layout property `{property}`\nhelp: expected {expected}\nhelp: use `{{...}}` to pass a Rust expression instead"
        ),
    )
}

pub(crate) fn deserialize_css<T>(value: &str) -> Option<T>
where
    T: for<'de> Deserialize<'de>,
{
    T::deserialize(BorrowedStrDeserializer::<SerdeValueError>::new(value)).ok()
}

pub(crate) fn css_value_from_tokens(tokens: &[TokenTree]) -> Option<String> {
    if let [TokenTree::Literal(literal)] = tokens {
        return Some(literal.to_string());
    }

    if let [TokenTree::Literal(literal), TokenTree::Punct(punct)] = tokens
        && punct.as_char() == '%'
    {
        return Some(format!("{}%", literal));
    }

    let mut out = String::new();
    let mut expect_ident = true;

    for token in tokens {
        match token {
            TokenTree::Ident(ident) if expect_ident => {
                if !out.is_empty() {
                    out.push('-');
                }
                out.push_str(&ident.to_string());
                expect_ident = false;
            }
            TokenTree::Punct(punct) if punct.as_char() == '-' && !expect_ident => {
                expect_ident = true;
            }
            _ => return None,
        }
    }

    (!out.is_empty() && !expect_ident).then_some(out)
}

fn starts_ident(tokens: &[TokenTree], ident: &str) -> bool {
    matches!(tokens.first(), Some(TokenTree::Ident(first)) if first == ident)
}

fn is_css_value(tokens: &[TokenTree]) -> bool {
    !tokens.is_empty()
        && tokens.iter().all(|token| {
            matches!(token, TokenTree::Ident(_) | TokenTree::Literal(_))
                || matches!(token, TokenTree::Punct(punct) if matches!(punct.as_char(), '-' | '%'))
        })
}

fn is_css_track_list(tokens: &[TokenTree]) -> bool {
    !tokens.is_empty()
        && tokens.iter().all(|token| {
            matches!(token, TokenTree::Ident(_) | TokenTree::Literal(_))
                || matches!(token, TokenTree::Punct(punct) if matches!(punct.as_char(), '-' | '%' | ','))
        })
}

#[cfg(test)]
mod tests {
    use quote::quote;

    use super::{Parser, Target};

    #[test]
    fn invalid_enum_value_explains_the_property_and_fix() {
        let error = Parser::new(quote! { direction: diagonal; }, Target::Layout)
            .parse()
            .expect_err("invalid direction should fail");
        let message = error.to_string();

        assert!(message.contains("invalid value `diagonal` for layout property `direction`"));
        assert!(message.contains("expected `horizontal` or `vertical`"));
        assert!(message.contains("use `{...}` to pass a Rust expression instead"));
    }

    #[test]
    fn invalid_track_value_explains_the_expected_syntax() {
        let error = Parser::new(quote! { columns: 1fr fill; }, Target::Layout)
            .parse()
            .expect_err("invalid track should fail");
        let message = error.to_string();

        assert!(message.contains("layout property `columns`"));
        assert!(message.contains("space- or comma-separated list"));
    }
}
