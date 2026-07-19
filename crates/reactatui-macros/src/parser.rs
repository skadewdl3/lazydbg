use proc_macro2::{Delimiter, Ident, Punct, Spacing, TokenStream as TokenStream2, TokenTree};

use crate::ast::{Element, ElseBranch, ForNode, IfNode, Node, Prop, Tag};

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

        if let Some(TokenTree::Group(group)) = self.peek().cloned() {
            if group.delimiter() == Delimiter::Brace {
                self.pos += 1;
                return Ok(Node::Expr(group.stream()));
            }
        }

        Ok(Node::Expr(self.collect_expression_child()))
    }

    fn parse_element_or_fragment(&mut self) -> syn::Result<Node> {
        self.expect_punct('<')?;
        if self.consume_punct('>') {
            let children = self.parse_nodes_until_fragment_close()?;
            return Ok(Node::Fragment(children));
        }

        let tag = self.parse_tag()?;
        let mut props = Vec::new();
        while !self.is_done() {
            if self.consume_punct('/') {
                self.expect_punct('>')?;
                return Ok(Node::Element(Element {
                    tag,
                    props,
                    children: Vec::new(),
                }));
            }

            if self.consume_punct('>') {
                let children = self.parse_nodes_until_close(Some(&tag))?;
                return Ok(Node::Element(Element {
                    tag,
                    props,
                    children,
                }));
            }

            props.push(self.parse_prop()?);
        }

        Err(self.error("unterminated element"))
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

        while self.consume_colon2() {
            let ident = self.expect_ident()?;
            if self.path_can_be_constructor(&path) && self.is_tag_boundary() {
                constructor = Some(ident);
                break;
            }
            path.push(ident);
        }

        Ok(Tag { path, constructor })
    }

    fn parse_prop(&mut self) -> syn::Result<Prop> {
        if self.consume_punct('.') {
            self.expect_punct('.')?;
            return Ok(Prop::Spread(self.collect_prop_expr()));
        }

        let name = self.expect_ident()?;

        // Detect `on:click` / `on:mousein` / `on:mouseout` / `on:scrollx` / `on:scrolly` — single colon (not `::`)
        if name == "on" {
            if matches!(self.peek(), Some(TokenTree::Punct(p)) if p.as_char() == ':') {
                // Make sure the NEXT token after `:` is NOT another `:` (that would be `::`)
                if !matches!(self.peek_n(1), Some(TokenTree::Punct(p)) if p.as_char() == ':') {
                    self.pos += 1; // consume `:`
                    let kind_ident = self.expect_ident()?;
                    let kind = kind_ident.to_string();
                    if !self.consume_punct('=') {
                        return Err(self.error(format!("on:{kind} requires a value: on:{kind}={{handler}}")))
                    }
                    let Some(TokenTree::Group(group)) = self.peek().cloned() else {
                        return Err(self.error("event handler must be wrapped in braces"));
                    };
                    if group.delimiter() != Delimiter::Brace {
                        return Err(self.error("event handler must be wrapped in braces"));
                    }
                    self.pos += 1;
                    return Ok(Prop::Event { kind, handler: group.stream() });
                }
            }
        }

        if !self.consume_punct('=') {
            return Ok(Prop::Boolean(name));
        }

        let Some(TokenTree::Group(group)) = self.peek().cloned() else {
            return Err(self.error("prop values must be wrapped in braces"));
        };
        if group.delimiter() != Delimiter::Brace {
            return Err(self.error("prop values must be wrapped in braces"));
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
        Err(self.error("expected a braced block"))
    }

    fn collect_prop_expr(&mut self) -> TokenStream2 {
        let mut out = TokenStream2::new();
        while !self.is_done() && !self.is_tag_boundary() {
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

    fn consume_closing_tag(&mut self, close_tag: Option<&Tag>) -> syn::Result<()> {
        self.expect_punct('<')?;
        self.expect_punct('/')?;
        let actual = self.parse_tag()?;
        self.expect_punct('>')?;
        if let Some(expected) = close_tag {
            if !expected.same_name(&actual) {
                return Err(self.error("closing tag does not match opening tag"));
            }
        }
        Ok(())
    }

    fn is_tag_boundary(&self) -> bool {
        matches!(self.peek(), Some(TokenTree::Punct(punct)) if matches!(punct.as_char(), '/' | '>'))
            || matches!(self.peek(), Some(TokenTree::Ident(_)))
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
}

impl Tag {
    pub fn same_name(&self, other: &Tag) -> bool {
        self.path.len() == other.path.len()
            && self.path.iter().zip(&other.path).all(|(a, b)| a == b)
    }

    pub fn simple_name(&self) -> Option<String> {
        (self.path.len() == 1 && self.constructor.is_none()).then(|| self.path[0].to_string())
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
