use proc_macro2::{Ident, TokenStream as TokenStream2};

#[derive(Clone)]
pub enum Node {
    Element(Element),
    Fragment(Vec<Node>),
    Expr(TokenStream2),
    If(IfNode),
    For(ForNode),
}

#[derive(Clone)]
pub struct Element {
    pub tag: Tag,
    pub props: Vec<Prop>,
    pub children: Vec<Node>,
}

#[derive(Clone)]
pub struct Tag {
    pub path: Vec<Ident>,
    pub constructor: Option<Ident>,
    /// Optional positional arguments passed via `(arg1, arg2)` syntax after the tag/constructor name.
    pub constructor_args: Option<TokenStream2>,
}

#[derive(Clone)]
pub enum Prop {
    Named { name: Ident, value: TokenStream2 },
    Boolean(Ident),
    Spread(TokenStream2),
    Event { kind: String, handler: TokenStream2 },
}

#[derive(Clone)]
pub struct IfNode {
    pub condition: TokenStream2,
    pub then_branch: Vec<Node>,
    pub else_branch: Option<ElseBranch>,
}

#[derive(Clone)]
pub enum ElseBranch {
    If(Box<IfNode>),
    Nodes(Vec<Node>),
}

#[derive(Clone)]
pub struct ForNode {
    pub head: TokenStream2,
    pub body: Vec<Node>,
}
