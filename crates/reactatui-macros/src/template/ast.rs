use proc_macro2::{Ident, TokenStream as TokenStream2};

#[derive(Clone)]
pub enum Node {
    Element(Element),
    // An "empty" element, that only acts as a container to other elements.
    Fragment(Vec<Node>),
    // A block expression. The return value must be an element.
    Expr(TokenStream2),
    If(IfNode),
    For(ForNode),
}

/// An element is either a normal ratatui widget or
/// a component written with JSX-like syntax where the
/// tag is the optionally qualified name + constructor
#[derive(Clone)]
pub struct Element {
    pub tag: Tag,
    pub props: Vec<Prop>,
    /// An element can have zero or more "children", which are nodes
    /// logically nested inside it. How the children are rendered
    /// Is up to the discretion of the element.
    pub children: Vec<Node>,
}

#[derive(Clone)]
pub struct Tag {
    /// Qualified path of the identifier
    pub path: Vec<Ident>,
    /// Constructor name of the tag can be customized
    /// or it defaults to `Tag::new()`
    pub constructor: Option<Ident>,
    /// Optional positional arguments passed via `(arg1, arg2)`
    /// syntax after the tag/constructor name.
    pub constructor_args: Option<TokenStream2>,
}

/// A prop is syntax sugar for method-chaining.
/// For example, `Block::default().border(Borders::ALL)` is
/// written as: `<Block::default border={Borders::ALL}> ... </Block>
#[derive(Clone)]
pub enum Prop {
    /// A standard prop - <Element named={...} />
    Named { name: Ident, value: TokenStream2 },
    /// A prop whose value defaults to true,
    /// if specified <Element is-active />
    Boolean(Ident),
    /// Shorthand for specifying all key-value pairs in a struct
    /// as named props, for ex: <Element ..{props} />
    Spread(TokenStream2),
    /// An event handler for an event that can be emitted by the Element
    /// For ex, <Button on:click={move || { ... }} />
    Event { kind: String, handler: TokenStream2 },
}

/// Syntax sugar for conditionally rendering an element
/// This differs from block expressions `{ ... }` because it
/// doesn't need the return types of all branches to match.
#[derive(Clone)]
pub struct IfNode {
    pub condition: TokenStream2,
    pub then_branch: Vec<Node>,
    pub else_branch: Option<ElseBranch>,
}

/// Else-counterpart of the `IfNode`. Can be followed by
/// more nodes to form if-else if-else chains.
#[derive(Clone)]
pub enum ElseBranch {
    If(Box<IfNode>),
    Nodes(Vec<Node>),
}

/// Syntax sugar for declaratively rendering a sequence of elements
/// based on an iterator, as opposed to returning a Fragment element
/// with a custom children array from a block expression.
#[derive(Clone)]
pub struct ForNode {
    pub head: TokenStream2,
    pub body: Vec<Node>,
}
