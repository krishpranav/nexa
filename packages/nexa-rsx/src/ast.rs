use proc_macro2::Span;
use syn::{Expr, Ident, LitStr};

pub struct RsxNodes {
    pub nodes: Vec<RsxNode>,
}

pub enum RsxNode {
    Element(Element),
    Component(Component),
    Text(LitStrOrExpr),
    Fragment(RsxNodes),
    ControlFlow(ControlFlow),
}

pub struct Element {
    pub name: Ident,
    pub attributes: Vec<Attribute>,
    pub children: Vec<RsxNode>,
    pub _span: Span,
}

pub struct Component {
    pub name: Ident,
    pub props: Vec<Prop>,
    pub children: Vec<RsxNode>, // Usually components don't have children in RSX unless via children prop
    pub _span: Span,
}

pub struct Attribute {
    pub name: Ident,
    pub value: AttributeValue,
}

pub enum AttributeValue {
    Lit(LitStr),
    Expr(Expr),
    Shorthand,
}

pub struct Prop {
    pub name: Ident,
    pub value: PropValue,
}

pub enum PropValue {
    Expr(Expr),
    Shorthand,
}

pub enum LitStrOrExpr {
    Lit(LitStr),
    Expr(Expr),
}

pub enum ControlFlow {
    If {
        cond: Expr,
        then_branch: RsxNodes,
        else_branch: Option<RsxNodes>,
    },
    For {
        pat: syn::Pat,
        expr: Expr,
        body: RsxNodes,
        key: Option<Expr>,
    },
}

impl RsxNode {
    pub fn is_static(&self) -> bool {
        match self {
            RsxNode::Element(el) => el.is_static(),
            RsxNode::Text(txt) => match txt {
                LitStrOrExpr::Lit(_) => true,
                LitStrOrExpr::Expr(_) => false,
            },
            RsxNode::Fragment(f) => f.nodes.iter().all(|n| n.is_static()),
            RsxNode::Component(_) => false,
            RsxNode::ControlFlow(_) => false,
        }
    }
}

impl Element {
    pub fn is_static(&self) -> bool {
        self.attributes.iter().all(|a| match a.value {
            AttributeValue::Lit(_) => true,
            _ => false,
        }) && self.children.iter().all(|c| c.is_static())
    }
}
