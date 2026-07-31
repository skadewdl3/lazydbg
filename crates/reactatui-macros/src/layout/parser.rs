//! Parser for CSS-like declaration blocks used by `layout!` and `style!`.
//!
//! This module handles syntax that is independent of any specific property:
//! rules, block `if` chains, top-level `match` blocks, and match-arm shorthand.
//! Property dispatch is delegated back to `layout::property_value`.

use proc_macro2::{Delimiter, Ident, TokenStream as TokenStream2, TokenTree};
use quote::{format_ident, quote};
use syn::parse::Parser as _;

use super::{RuleTokens, property_value};

#[derive(Clone, Copy)]
pub(crate) enum Target {
    Layout,
    Style,
}

pub(crate) struct Parser {
    pub(crate) tokens: Vec<TokenTree>,
    pub(crate) pos: usize,
    pub(crate) target: Target,
}

#[derive(Clone)]
pub(crate) struct CssName {
    parts: Vec<Ident>,
}

impl Parser {
    pub(crate) fn new(tokens: TokenStream2, target: Target) -> Self {
        Self {
            tokens: tokens.into_iter().collect(),
            pos: 0,
            target,
        }
    }

    pub(crate) fn parse(&mut self) -> syn::Result<TokenStream2> {
        let style = match self.target {
            Target::Layout => format_ident!("__reactatui_layout_style"),
            Target::Style => format_ident!("__reactatui_style"),
        };
        let body = self.parse_style_block(&style)?;
        let initial = match self.target {
            Target::Layout => quote! { ::reactatui::layout::Style::default() },
            Target::Style => quote! { ::reactatui::ReactatuiStyle::new() },
        };
        Ok(quote! {{
            let mut #style = #initial;
            #body
            #style
        }})
    }

    pub(crate) fn parse_style_block(&mut self, style: &Ident) -> syn::Result<TokenStream2> {
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

    pub(crate) fn collect_until_brace_group(
        &mut self,
    ) -> syn::Result<(TokenStream2, TokenStream2)> {
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

    pub(crate) fn expect_brace_group(&mut self) -> syn::Result<TokenStream2> {
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

    pub(crate) fn expect_keyword(&mut self, keyword: &str) -> syn::Result<()> {
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

    pub(crate) fn peek_ident(&self, name: &str) -> bool {
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

    pub(crate) fn is_done(&self) -> bool {
        self.pos >= self.tokens.len()
    }

    pub(crate) fn error(&self, message: impl std::fmt::Display) -> syn::Error {
        let span = self
            .peek()
            .map(TokenTree::span)
            .unwrap_or_else(proc_macro2::Span::call_site);
        syn::Error::new(span, message)
    }
}

impl CssName {
    pub(crate) fn first_part(&self) -> &Ident {
        self.parts.first().expect("css names are non-empty")
    }

    pub(crate) fn as_kebab(&self) -> String {
        self.parts
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("-")
    }
}

impl Target {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Layout => "layout",
            Self::Style => "style",
        }
    }
}

pub(crate) fn parse_style(input: TokenStream2) -> syn::Result<TokenStream2> {
    Parser::new(input, Target::Style).parse()
}
