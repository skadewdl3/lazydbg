use proc_macro2::{Delimiter, Ident, Punct, Spacing, TokenStream as TokenStream2, TokenTree};
use syn::parse::Parser as _;

use crate::template::ast::{
    Element, ElseBranch, ForNode, IfNode, MatchArm, MatchNode, Node, Prop, Tag,
};

pub struct Parser {
    tokens: Vec<TokenTree>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: TokenStream2) -> Self {
        Self {
            tokens: tokens.into_iter().collect(),
            pos: 0,
        }
    }

    pub fn parse_nodes_until_close(&mut self, close_tag: Option<&Tag>) -> syn::Result<Vec<Node>> {
        let mut nodes = Vec::new();
        while !self.is_done() {
            if self.starts_closing_tag(close_tag) {
                self.consume_closing_tag(close_tag)?;
                break;
            }

            if self.starts_fragment_close() {
                break;
            }

            nodes.push(self.parse_node()?);
        }
        Ok(nodes)
    }

    fn parse_node(&mut self) -> syn::Result<Node> {
        if self.starts_element() {
            return self.parse_element_or_fragment();
        }

        if self.peek_ident("if") {
            return self.parse_if().map(Node::If);
        }

        if self.peek_ident("for") {
            return self.parse_for().map(Node::For);
        }

        if self.peek_ident("match") {
            return self.parse_match().map(Node::Match);
        }

        if let Some(TokenTree::Group(group)) = self.peek().cloned()
            && group.delimiter() == Delimiter::Brace
        {
            let mut inner = Parser::new(group.stream());
            if inner.peek_ident("for") {
                self.pos += 1;
                return inner.parse_for().map(Node::For);
            }
            if inner.peek_ident("if") {
                self.pos += 1;
                return inner.parse_if().map(Node::If);
            }
            if inner.peek_ident("match") {
                self.pos += 1;
                return inner.parse_match().map(Node::Match);
            }
            if inner.starts_element() {
                self.pos += 1;
                return inner.parse_element_or_fragment();
            }
            self.pos += 1;
            return Ok(Node::Expr(group.stream()));
        }

        Ok(Node::Expr(self.collect_expression_child()))
    }

    fn parse_element_or_fragment(&mut self) -> syn::Result<Node> {
        self.expect_punct('<')?;
        if self.consume_punct('>') {
            let children = self.parse_nodes_until_fragment_close()?;
            return Ok(Node::Fragment(children));
        }

        let tag = if let Some(TokenTree::Group(group)) = self.peek().cloned()
            && group.delimiter() == Delimiter::Brace
        {
            self.pos += 1;
            if group.stream().is_empty() {
                return Err(self.error("dynamic component expressions cannot be empty"));
            }
            Tag {
                dynamic: Some(group.stream()),
                path: Vec::new(),
                constructor: None,
                constructor_args: None,
            }
        } else {
            self.parse_tag()?
        };
        let mut props = Vec::new();
        while !self.is_done() {
            if self.consume_punct('/') {
                self.expect_punct('>')?;
                return Ok(Node::Element(Element {
                    tag,
                    props,
                    children: Vec::new(),
                    slots: Vec::new(),
                }));
            }

            if self.consume_punct('>') {
                if tag.dynamic.is_some() {
                    return Err(self.error(
                        "dynamic components must be self-closing\nhelp: write `<{node} ... />`",
                    ));
                }
                let (children, slots) = self.parse_element_children_until_close(&tag)?;
                return Ok(Node::Element(Element {
                    tag,
                    props,
                    children,
                    slots,
                }));
            }

            let prop = self.parse_prop()?;
            props.push(prop);
        }

        if tag.dynamic.is_some() {
            Err(self.error("dynamic components must be self-closing\nhelp: write `<{node} ... />`"))
        } else {
            Err(self.error_spanned(
                tag.path.first().expect("tags have a name"),
                format!(
                    "unclosed `<{}>` element\nhelp: add `/>` for an empty element or close it with `</{}>`",
                    tag.type_name(),
                    tag.type_name(),
                ),
            ))
        }
    }

    fn parse_element_children_until_close(
        &mut self,
        close_tag: &Tag,
    ) -> syn::Result<(Vec<Node>, Vec<(Ident, Vec<Node>)>)> {
        let mut children = Vec::new();
        let mut slots = Vec::new();
        while !self.is_done() {
            if self.starts_closing_tag(Some(close_tag)) {
                self.consume_closing_tag(Some(close_tag))?;
                return Ok((children, slots));
            }

            if self.starts_slot_open() {
                slots.push(self.parse_slot()?);
            } else {
                children.push(self.parse_node()?);
            }
        }
        Err(self.error_spanned(
            close_tag.path.first().expect("tags have a name"),
            format!(
                "unclosed `<{}>` element\nhelp: add a matching `</{}>` closing tag",
                close_tag.type_name(),
                close_tag.type_name(),
            ),
        ))
    }

    fn parse_nodes_until_fragment_close(&mut self) -> syn::Result<Vec<Node>> {
        let mut nodes = Vec::new();
        while !self.is_done() {
            if self.starts_fragment_close() {
                self.expect_punct('<')?;
                self.expect_punct('/')?;
                self.expect_punct('>')?;
                return Ok(nodes);
            }
            nodes.push(self.parse_node()?);
        }
        Err(self.error("unterminated fragment"))
    }

    fn parse_tag(&mut self) -> syn::Result<Tag> {
        let mut path = vec![self.expect_ident()?];
        let mut constructor = None;
        let mut constructor_args = None;

        while self.consume_colon2() {
            let ident = self.expect_ident()?;
            if self.path_can_be_constructor(&path) && self.is_tag_boundary() {
                constructor = Some(ident);
                break;
            }
            path.push(ident);
        }

        // Parse optional positional args: `(arg1, arg2)` immediately after the tag/constructor name.
        if let Some(TokenTree::Group(group)) = self.peek().cloned()
            && group.delimiter() == Delimiter::Parenthesis
        {
            self.pos += 1;
            constructor_args = Some(group.stream());
        }

        Ok(Tag {
            dynamic: None,
            path,
            constructor,
            constructor_args,
        })
    }

    fn parse_prop(&mut self) -> syn::Result<Prop> {
        if let Some(TokenTree::Group(group)) = self.peek().cloned()
            && group.delimiter() == Delimiter::Brace
        {
            self.pos += 1;
            let mut inner = Parser::new(group.stream());
            if inner.consume_punct('.') {
                inner.expect_punct('.')?;
                let value = inner.collect_remaining();
                if value.is_empty() {
                    return Err(self.error(
                        "spread props require an expression after `..`\nhelp: write `{..props}`",
                    ));
                }
                return Ok(Prop::Spread(value));
            }

            let name = inner.expect_ident()?;
            if !inner.is_done() {
                return Err(self.error("attribute shorthand must be a single identifier\nhelp: use `{enabled}` or `name={value}`"));
            }
            return Ok(Prop::Named {
                name: name.clone(),
                value: quote::quote! { #name },
            });
        }

        if self.consume_punct('.') {
            self.expect_punct('.')?;
            return Ok(Prop::Spread(self.collect_prop_expr()));
        }

        let name = self.expect_ident()?;

        if (name == "on" || name == "bind")
            && matches!(self.peek(), Some(TokenTree::Punct(punct)) if punct.as_char() == ':')
            && matches!(self.peek_n(1), Some(TokenTree::Punct(punct)) if punct.as_char() == ':')
        {
            return Err(self.error_spanned(
                &name,
                format!(
                    "`{name}` attributes use one colon, not a Rust path\nhelp: write `{name}:value={{...}}`"
                ),
            ));
        }

        // Detect `on:click` / `on:mousein` / `on:mouseout` / `on:scrollx` / `on:scrolly` — single colon (not `::`)
        if name == "on" && matches!(self.peek(), Some(TokenTree::Punct(p)) if p.as_char() == ':') {
            // Make sure the NEXT token after `:` is NOT another `:` (that would be `::`)
            if !matches!(self.peek_n(1), Some(TokenTree::Punct(p)) if p.as_char() == ':') {
                self.pos += 1; // consume `:`
                let kind_ident = self.expect_ident()?;
                let kind = kind_ident.to_string();
                if !self.consume_punct('=') {
                    return Err(
                        self.error(format!("on:{kind} requires a value: on:{kind}={{handler}}"))
                    );
                }
                let Some(TokenTree::Group(group)) = self.peek().cloned() else {
                    return Err(self.error_spanned(
                        &kind_ident,
                        format!(
                            "event handler for `on:{kind}` must be wrapped in braces\nhelp: write `on:{kind}={{handler}}`"
                        ),
                    ));
                };
                if group.delimiter() != Delimiter::Brace {
                    return Err(self.error_spanned(
                        &kind_ident,
                        format!(
                            "event handler for `on:{kind}` must be wrapped in braces\nhelp: write `on:{kind}={{handler}}`"
                        ),
                    ));
                }
                self.pos += 1;
                return Ok(Prop::Event {
                    kind,
                    handler: group.stream(),
                });
            }
        }

        if name == "bind" {
            let bind_name = if matches!(self.peek(), Some(TokenTree::Punct(p)) if p.as_char() == ':')
                && !matches!(self.peek_n(1), Some(TokenTree::Punct(p)) if p.as_char() == ':')
            {
                self.pos += 1;
                Some(self.expect_ident()?)
            } else {
                None
            };

            if !self.consume_punct('=') {
                return Err(self.error_spanned(
                    &name,
                    "binding requires a braced state expression\nhelp: write `bind={state}` or `bind:name={state}`",
                ));
            }
            let Some(TokenTree::Group(group)) = self.peek().cloned() else {
                return Err(self.error_spanned(
                    &name,
                    "binding value must be wrapped in braces\nhelp: write `bind={state}` or `bind:name={state}`",
                ));
            };
            if group.delimiter() != Delimiter::Brace {
                return Err(self.error_spanned(
                    &name,
                    "binding value must be wrapped in braces\nhelp: write `bind={state}` or `bind:name={state}`",
                ));
            }
            self.pos += 1;
            return Ok(Prop::Bind {
                name: bind_name,
                value: group.stream(),
            });
        }

        if !self.consume_punct('=') {
            return Ok(Prop::Boolean(name));
        }

        let Some(TokenTree::Group(group)) = self.peek().cloned() else {
            return Err(self.error_spanned(
                &name,
                format!(
                    "value for prop `{name}` must be wrapped in braces\nhelp: write `{name}={{value}}`"
                ),
            ));
        };
        if group.delimiter() != Delimiter::Brace {
            return Err(self.error_spanned(
                &name,
                format!(
                    "value for prop `{name}` must be wrapped in braces\nhelp: write `{name}={{value}}`"
                ),
            ));
        }
        self.pos += 1;

        Ok(Prop::Named {
            name,
            value: group.stream(),
        })
    }

    fn parse_if(&mut self) -> syn::Result<IfNode> {
        self.expect_keyword("if")?;
        let (condition, body) = self.collect_until_brace_group()?;
        let then_branch = Parser::new(body).parse_nodes_until_close(None)?;
        let else_branch = if self.peek_ident("else") {
            self.expect_keyword("else")?;
            if self.peek_ident("if") {
                Some(ElseBranch::If(Box::new(self.parse_if()?)))
            } else {
                let Some(TokenTree::Group(group)) = self.peek().cloned() else {
                    return Err(self.error("else branch must use braces"));
                };
                if group.delimiter() != Delimiter::Brace {
                    return Err(self.error("else branch must use braces"));
                }
                self.pos += 1;
                Some(ElseBranch::Nodes(
                    Parser::new(group.stream()).parse_nodes_until_close(None)?,
                ))
            }
        } else {
            None
        };

        Ok(IfNode {
            condition,
            then_branch,
            else_branch,
        })
    }

    fn parse_for(&mut self) -> syn::Result<ForNode> {
        self.expect_keyword("for")?;
        let (head, body) = self.collect_until_brace_group()?;
        Ok(ForNode {
            head,
            body: Parser::new(body).parse_nodes_until_close(None)?,
        })
    }

    fn parse_match(&mut self) -> syn::Result<MatchNode> {
        self.expect_keyword("match")?;
        let (scrutinee, body) = self.collect_until_brace_group()?;
        let mut inner = Parser::new(body);
        let mut arms = Vec::new();
        while !inner.is_done() {
            arms.push(inner.parse_match_arm()?);
        }
        Ok(MatchNode { scrutinee, arms })
    }

    fn parse_match_arm(&mut self) -> syn::Result<MatchArm> {
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

        let (body, comma_required) = self.parse_match_arm_body()?;

        let has_comma = self.consume_punct(',');
        if comma_required && !has_comma && !self.is_done() {
            return Err(self.error("match arm expression bodies require a trailing comma"));
        }
        Ok(MatchArm {
            pattern,
            guard,
            body,
        })
    }

    fn parse_match_arm_body(&mut self) -> syn::Result<(Vec<Node>, bool)> {
        if let Some(TokenTree::Group(group)) = self.peek().cloned()
            && group.delimiter() == Delimiter::Brace
        {
            self.pos += 1;
            return Ok((
                Parser::new(group.stream()).parse_nodes_until_close(None)?,
                true,
            ));
        }

        if self.starts_element()
            || self.peek_ident("if")
            || self.peek_ident("for")
            || self.peek_ident("match")
        {
            return Ok((vec![self.parse_node()?], false));
        }

        if let Some(TokenTree::Group(group)) = self.peek().cloned()
            && group.delimiter() == Delimiter::Brace
        {
            self.pos += 1;
            return Ok((vec![Node::Expr(group.stream())], true));
        }

        let mut expr = TokenStream2::new();
        while !self.is_done() && !self.peek_punct(',') {
            expr.extend([self.tokens[self.pos].clone()]);
            self.pos += 1;
        }
        if expr.is_empty() {
            return Err(self.error("match arm requires a body"));
        }
        Ok((vec![Node::Expr(expr)], true))
    }

    fn collect_expression_child(&mut self) -> TokenStream2 {
        let mut out = TokenStream2::new();
        while !self.is_done() {
            if self.starts_element() || self.starts_fragment_close() {
                break;
            }
            if self.peek_ident("else") {
                break;
            }
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
        Err(self.error("expected a braced block\nhelp: wrap the body in `{ ... }`"))
    }

    fn collect_prop_expr(&mut self) -> TokenStream2 {
        let mut out = TokenStream2::new();
        while !self.is_done() && !self.is_tag_boundary() {
            out.extend([self.tokens[self.pos].clone()]);
            self.pos += 1;
        }
        out
    }

    fn collect_remaining(&mut self) -> TokenStream2 {
        let mut out = TokenStream2::new();
        while !self.is_done() {
            out.extend([self.tokens[self.pos].clone()]);
            self.pos += 1;
        }
        out
    }

    fn starts_element(&self) -> bool {
        matches!(self.peek(), Some(TokenTree::Punct(punct)) if punct.as_char() == '<')
    }

    fn starts_closing_tag(&self, close_tag: Option<&Tag>) -> bool {
        close_tag.is_some()
            && matches!(self.peek(), Some(TokenTree::Punct(punct)) if punct.as_char() == '<')
            && matches!(self.peek_n(1), Some(TokenTree::Punct(punct)) if punct.as_char() == '/')
    }

    fn starts_fragment_close(&self) -> bool {
        matches!(self.peek(), Some(TokenTree::Punct(punct)) if punct.as_char() == '<')
            && matches!(self.peek_n(1), Some(TokenTree::Punct(punct)) if punct.as_char() == '/')
            && matches!(self.peek_n(2), Some(TokenTree::Punct(punct)) if punct.as_char() == '>')
    }

    fn starts_slot_open(&self) -> bool {
        matches!(self.peek(), Some(TokenTree::Punct(punct)) if punct.as_char() == '<')
            && matches!(self.peek_n(1), Some(TokenTree::Ident(ident)) if ident == "slot")
            && matches!(self.peek_n(2), Some(TokenTree::Punct(punct)) if punct.as_char() == ':')
            && !matches!(self.peek_n(3), Some(TokenTree::Punct(punct)) if punct.as_char() == ':')
    }

    fn starts_slot_close(&self) -> bool {
        matches!(self.peek(), Some(TokenTree::Punct(punct)) if punct.as_char() == '<')
            && matches!(self.peek_n(1), Some(TokenTree::Punct(punct)) if punct.as_char() == '/')
            && matches!(self.peek_n(2), Some(TokenTree::Ident(ident)) if ident == "slot")
            && matches!(self.peek_n(3), Some(TokenTree::Punct(punct)) if punct.as_char() == ':')
            && !matches!(self.peek_n(4), Some(TokenTree::Punct(punct)) if punct.as_char() == ':')
    }

    fn parse_slot(&mut self) -> syn::Result<(Ident, Vec<Node>)> {
        self.expect_punct('<')?;
        self.expect_keyword("slot")?;
        self.expect_single_colon()?;
        let name = self.expect_ident()?;
        self.expect_punct('>')?;

        let mut children = Vec::new();
        while !self.is_done() {
            if self.starts_slot_close() {
                self.expect_punct('<')?;
                self.expect_punct('/')?;
                self.expect_keyword("slot")?;
                self.expect_single_colon()?;
                let close_name = self.expect_ident()?;
                self.expect_punct('>')?;
                if close_name != name {
                    return Err(self.error_spanned(
                        &close_name,
                        format!(
                            "slot closing tag `</slot:{close_name}>` does not match `<slot:{name}>`\nhelp: close the slot with `</slot:{name}>`"
                        ),
                    ));
                }
                return Ok((name, children));
            }
            children.push(self.parse_node()?);
        }
        Err(self.error("unterminated slot"))
    }

    fn consume_closing_tag(&mut self, close_tag: Option<&Tag>) -> syn::Result<()> {
        self.expect_punct('<')?;
        self.expect_punct('/')?;
        let actual = self.parse_tag()?;
        self.expect_punct('>')?;
        if let Some(expected) = close_tag
            && !expected.same_name(&actual)
        {
            return Err(self.error_spanned(
                actual.path.first().expect("tags have a name"),
                format!(
                    "closing tag `</{}>` does not match `<{}>`\nhelp: replace it with `</{}>`",
                    actual.type_name(),
                    expected.type_name(),
                    expected.type_name(),
                ),
            ));
        }
        Ok(())
    }

    fn is_tag_boundary(&self) -> bool {
        matches!(self.peek(), Some(TokenTree::Punct(punct)) if matches!(punct.as_char(), '/' | '>'))
            || matches!(self.peek(), Some(TokenTree::Ident(_)))
            || matches!(self.peek(), Some(TokenTree::Group(g)) if g.delimiter() == Delimiter::Parenthesis)
    }

    fn path_can_be_constructor(&self, path: &[Ident]) -> bool {
        path.len() == 1
    }

    fn expect_keyword(&mut self, keyword: &str) -> syn::Result<()> {
        if self.peek_ident(keyword) {
            self.pos += 1;
            Ok(())
        } else {
            Err(self.error(format!("expected `{keyword}`")))
        }
    }

    fn expect_ident(&mut self) -> syn::Result<Ident> {
        match self.peek().cloned() {
            Some(TokenTree::Ident(ident)) => {
                self.pos += 1;
                Ok(ident)
            }
            _ => Err(self.error("expected identifier")),
        }
    }

    fn expect_punct(&mut self, ch: char) -> syn::Result<()> {
        if self.consume_punct(ch) {
            Ok(())
        } else {
            Err(self.error(format!("expected `{ch}`")))
        }
    }

    fn peek_punct(&self, ch: char) -> bool {
        matches!(self.peek(), Some(TokenTree::Punct(punct)) if punct.as_char() == ch)
    }

    fn expect_single_colon(&mut self) -> syn::Result<()> {
        if matches!(self.peek(), Some(TokenTree::Punct(punct)) if punct.as_char() == ':')
            && !matches!(self.peek_n(1), Some(TokenTree::Punct(punct)) if punct.as_char() == ':')
        {
            self.pos += 1;
            Ok(())
        } else {
            Err(self.error("expected `:`"))
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

    fn consume_punct(&mut self, ch: char) -> bool {
        if matches!(self.peek(), Some(TokenTree::Punct(punct)) if punct.as_char() == ch) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn consume_colon2(&mut self) -> bool {
        if matches!(self.peek(), Some(TokenTree::Punct(punct)) if punct.as_char() == ':')
            && matches!(self.peek_n(1), Some(TokenTree::Punct(punct)) if punct.as_char() == ':')
        {
            self.pos += 2;
            true
        } else {
            false
        }
    }

    fn peek_ident(&self, name: &str) -> bool {
        matches!(self.peek(), Some(TokenTree::Ident(ident)) if ident == name)
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

    fn error_spanned(
        &self,
        tokens: impl quote::ToTokens,
        message: impl std::fmt::Display,
    ) -> syn::Error {
        syn::Error::new_spanned(tokens, message)
    }
}

impl Tag {
    pub fn same_name(&self, other: &Tag) -> bool {
        if self.dynamic.is_some() || other.dynamic.is_some() {
            return false;
        }
        self.path.len() == other.path.len()
            && self.path.iter().zip(&other.path).all(|(a, b)| a == b)
    }

    pub fn root_name(&self) -> Option<String> {
        (self.path.len() == 1).then(|| self.path[0].to_string())
    }

    pub fn type_name(&self) -> String {
        self.path
            .last()
            .map(ToString::to_string)
            .unwrap_or_else(|| String::from(""))
    }

    pub fn type_path_tokens(&self) -> TokenStream2 {
        let mut out = TokenStream2::new();
        for (index, segment) in self.path.iter().enumerate() {
            if index > 0 {
                out.extend([
                    TokenTree::Punct(Punct::new(':', Spacing::Joint)),
                    TokenTree::Punct(Punct::new(':', Spacing::Alone)),
                ]);
            }
            out.extend([TokenTree::Ident(segment.clone())]);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use quote::quote;

    use super::Parser;

    #[test]
    fn prop_value_error_suggests_brace_syntax() {
        let result =
            Parser::new(quote! { <Block title="heading" /> }).parse_nodes_until_close(None);
        let Err(error) = result else {
            panic!("unbraced prop value should fail");
        };
        let message = error.to_string();

        assert!(message.contains("value for prop `title` must be wrapped in braces"));
        assert!(message.contains("write `title={value}`"));
    }

    #[test]
    fn mismatched_tag_error_names_the_expected_closer() {
        let result = Parser::new(quote! { <Block></Paragraph> }).parse_nodes_until_close(None);
        let Err(error) = result else {
            panic!("mismatched closing tag should fail");
        };
        let message = error.to_string();

        assert!(message.contains("closing tag `</Paragraph>` does not match `<Block>`"));
        assert!(message.contains("replace it with `</Block>`"));
    }

    #[test]
    fn event_attributes_reject_rust_path_syntax() {
        let result =
            Parser::new(quote! { <Button on::click={handler} /> }).parse_nodes_until_close(None);
        let Err(error) = result else {
            panic!("event attribute path syntax should fail");
        };
        let message = error.to_string();

        assert!(message.contains("attributes use one colon, not a Rust path"));
        assert!(message.contains("write `on:value={...}`"));
    }
}
