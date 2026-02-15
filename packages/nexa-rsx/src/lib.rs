use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{ToTokens, quote};
use std::collections::HashSet;
use syn::{
    Expr, Ident, LitStr, Result, Token,
    parse::{Parse, ParseStream},
    parse_macro_input,
    ext::IdentExt,
};

#[proc_macro]
pub fn rsx(input: TokenStream) -> TokenStream {
    let nodes = parse_macro_input!(input as RsxNodes);
    nodes.to_token_stream().into()
}

struct RsxNodes {
    pub nodes: Vec<RsxNode>,
}

impl Parse for RsxNodes {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut nodes = Vec::new();
        while !input.is_empty() {
            nodes.push(input.parse()?);
            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }
        Ok(RsxNodes { nodes })
    }
}

impl ToTokens for RsxNodes {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let nodes = &self.nodes;
        tokens.extend(quote! {
            {
                let mut __nodes: smallvec::SmallVec<[nexa_core::NodeId; 4]> = smallvec::SmallVec::new();
                #( #nodes )*
                __nodes
            }
        });
    }
}

enum RsxNode {
    Element(Element),
    Component(Component),
    Text(LitStrOrExpr),
    Fragment(RsxNodes),
    ControlFlow(ControlFlow),
}

impl RsxNode {
    fn is_static(&self) -> bool {
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

impl Parse for RsxNode {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.peek(Token![if]) || input.peek(Token![for]) {
            Ok(RsxNode::ControlFlow(input.parse()?))
        } else if input.peek(Token![<]) {
            // Fragment or Component?
            if input.peek2(Token![>]) {
                // Fragment: <> ... </>
                input.parse::<Token![<]>()?;
                input.parse::<Token![>]>()?;
                let content: RsxNodes = input.parse()?;
                input.parse::<Token![<]>()?;
                input.parse::<Token![/]>()?;
                input.parse::<Token![>]>()?;
                Ok(RsxNode::Fragment(content))
            } else {
                // Could be component or element based on capitalization but let's use standard tag/Ident
                Err(input.error("Expected element, component or block"))
            }
        } else if input.peek(syn::token::Brace) {
            // Block/Dynamic node
            let content;
            syn::braced!(content in input);
            Ok(RsxNode::Text(LitStrOrExpr::Expr(content.parse()?)))
        } else if input.peek(LitStr) {
            Ok(RsxNode::Text(LitStrOrExpr::Lit(input.parse()?)))
        } else {
            // Assuming Ident (element or component)
            let name: Ident = input.fork().parse()?;
            if name.to_string().chars().next().unwrap().is_uppercase() {
                Ok(RsxNode::Component(input.parse()?))
            } else {
                Ok(RsxNode::Element(input.parse()?))
            }
        }
    }
}

impl ToTokens for RsxNode {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        match self {
            RsxNode::Element(el) => el.to_tokens(tokens),
            RsxNode::Component(comp) => comp.to_tokens(tokens),
            RsxNode::Text(txt) => {
                let node = match txt {
                    LitStrOrExpr::Lit(l) => {
                        let s = l.value();
                        quote! {
                            nexa_core::VirtualNode::Text(nexa_core::Text {
                                text: #s.to_string(),
                                parent: None,
                            })
                        }
                    }
                    LitStrOrExpr::Expr(e) => {
                        quote! {
                            nexa_core::VirtualNode::Text(nexa_core::Text {
                                text: format!("{}", #e),
                                parent: None,
                            })
                        }
                    }
                };
                tokens.extend(quote! {
                    nexa_core::get_active_arena(|arena| {
                        __nodes.push(arena.insert(#node));
                    });
                });
            }
            RsxNode::Fragment(f) => {
                tokens.extend(quote! {
                    let mut __frag = #f;
                    __nodes.extend(__frag);
                });
            }
            RsxNode::ControlFlow(cf) => cf.to_tokens(tokens),
        }
    }
}

struct Element {
    pub name: Ident,
    pub attributes: Vec<Attribute>,
    pub children: Vec<RsxNode>,
    pub _span: Span,
}

impl Element {
    fn is_static(&self) -> bool {
        self.attributes.iter().all(|a| match a.value {
            AttributeValue::Lit(_) => true,
            _ => false,
        }) && self.children.iter().all(|c| c.is_static())
    }
}

impl Parse for Element {
    fn parse(input: ParseStream) -> Result<Self> {
        let name: Ident = input.parse()?;
        let span = name.span();
        let mut attributes: Vec<Attribute> = Vec::new();
        let mut children = Vec::new();

        if input.peek(syn::token::Brace) {
            let content;
            syn::braced!(content in input);
            while !content.is_empty() {
                if content.peek(LitStr) {
                    children.push(RsxNode::Text(LitStrOrExpr::Lit(content.parse()?)));
                } else if content.peek(syn::token::Brace) {
                    let block_content;
                    syn::braced!(block_content in content);
                    children.push(RsxNode::Text(LitStrOrExpr::Expr(block_content.parse()?)));
                } else if content.peek(Token![if]) || content.peek(Token![for]) {
                    children.push(RsxNode::ControlFlow(content.parse()?));
                } else if content.fork().call(Ident::parse_any).is_ok() {
                    // Ambiguous case: Attribute or Element?
                    // Use fork to peek ahead
                    let fork = content.fork();
                    let _ident = fork.call(Ident::parse_any)?;
                    
                    let is_attribute = if fork.peek(Token![:]) {
                        true // key: value
                    } else if fork.peek(syn::token::Brace) {
                        false // key { ... } -> Element/Component
                    } else {
                        true // key (shorthand) or key,
                    };

                    if is_attribute {
                        attributes.push(content.parse()?);
                    } else {
                        // Element or Component
                        // If capitalizing, Component. But RsxNode parse handles that.
                        // We can just call RsxNode::parse
                        children.push(content.parse()?);
                    }
                } else {
                    return Err(content.error("Unexpected token in RSX element"));
                }

                if content.peek(Token![,]) {
                    content.parse::<Token![,]>()?;
                }
            }
        }

        // Validate keys
        let mut keys = HashSet::new();
        for attr in &attributes {
             if attr.name == "key" {
                 if let AttributeValue::Lit(l) = &attr.value {
                     if !keys.insert(l.value()) {
                         return Err(syn::Error::new(
                             attr.name.span(),
                             "Duplicate key in element",
                         ));
                     }
                 }
             }
        }

        Ok(Element {
            name,
            attributes,
            children,
            _span: span,
        })
    }
}

impl ToTokens for Element {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let tag = self.name.to_string();
        let children = &self.children;
        let mut props = Vec::new();
        let mut listeners = Vec::new();

        for attr in &self.attributes {
            let name_str = attr.name.to_string();
            
            if name_str.starts_with("on") {
                // Event Listener
                // nexa-core::EventListener { name: "click", cb: Rc::new(RefCell::new(...)) }
                let event_name = name_str.trim_start_matches("on").to_lowercase();
                let val = match &attr.value {
                    AttributeValue::Lit(_) => {
                         // We don't support string literals for events yet, or maybe we do as "dynamic" handler lookup?
                         // For now, assume it's an expression or shorthand
                         quote! { panic!("String literal event handlers not supported yet in RSX") }
                    },
                    AttributeValue::Expr(e) => {
                        quote! { #e }
                    },
                    AttributeValue::Shorthand => {
                        let name_ident = &attr.name;
                        quote! { #name_ident }
                    }
                };
                
                listeners.push(quote! {
                    nexa_core::vdom::EventListener {
                        name: #event_name,
                        cb: std::rc::Rc::new(std::cell::RefCell::new(#val)),
                    }
                });
            } else {
                // Regular Attribute
                let val = match &attr.value {
                    AttributeValue::Lit(l) => {
                        let s = l.value();
                        quote! { #s.to_string() }
                    }
                    AttributeValue::Expr(e) => {
                        quote! { format!("{}", #e) }
                    }
                    AttributeValue::Shorthand => {
                        let name_ident = &attr.name;
                        quote! { format!("{}", #name_ident) }
                    }
                };
                props.push(quote! {
                    nexa_core::Attribute {
                        name: #name_str,
                        value: #val,
                    }
                });
            }
        }

        let is_static = self.is_static();
        let metadata = if is_static {
            quote! { nexa_core::NodeMetadata { is_static: true, render_count: 0 } }
        } else {
            quote! { nexa_core::NodeMetadata::default() }
        };

        tokens.extend(quote! {
            nexa_core::get_active_arena(|arena| {
                let mut __el_nodes: smallvec::SmallVec<[nexa_core::NodeId; 4]> = smallvec::SmallVec::new();
                {
                    let mut __nodes = __el_nodes;
                    #( #children )*
                    __el_nodes = __nodes;
                }
                
                let id = arena.insert_with_metadata(
                    nexa_core::VirtualNode::Element(nexa_core::Element {
                        tag: #tag,
                        props: smallvec::smallvec![ #(#props),* ],
                        listeners: smallvec::smallvec![ #(#listeners),* ],
                        children: __el_nodes,
                        parent: None,
                        key: None,
                    }),
                    #metadata
                );
                __nodes.push(id);
            });
        });
    }
}

struct Component {
    pub name: Ident,
    pub _attributes: Vec<Attribute>,
}

impl Parse for Component {
    fn parse(input: ParseStream) -> Result<Self> {
        let name: Ident = input.parse()?;
        let mut attributes = Vec::new();
        while input.peek(Ident) {
            attributes.push(input.parse()?);
            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }
        Ok(Component {
            name,
            _attributes: attributes,
        })
    }
}

impl ToTokens for Component {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let name = &self.name;
        // In Nexa, components are currently functions returning NodeId
        tokens.extend(quote! {
            __nodes.push(#name());
        });
    }
}

struct Attribute {
    pub name: Ident,
    pub value: AttributeValue,
}

impl Parse for Attribute {
    fn parse(input: ParseStream) -> Result<Self> {
        use syn::ext::IdentExt;
        let name: Ident = input.call(Ident::parse_any)?;
        if input.peek(Token![:]) {
            input.parse::<Token![:]>()?;
            if input.peek(LitStr) {
                Ok(Attribute {
                    name,
                    value: AttributeValue::Lit(input.parse()?),
                })
            } else {
                Ok(Attribute {
                    name,
                    value: AttributeValue::Expr(input.parse()?),
                })
            }
        } else {
            // Shorthand
            Ok(Attribute {
                name,
                value: AttributeValue::Shorthand,
            })
        }
    }
}

enum AttributeValue {
    Lit(LitStr),
    Expr(Expr),
    Shorthand,
}

enum LitStrOrExpr {
    Lit(LitStr),
    Expr(Expr),
}

enum ControlFlow {
    If {
        cond: Expr,
        then_branch: RsxNodes,
        else_branch: Option<RsxNodes>,
    },
    For {
        pat: syn::Pat,
        expr: Expr,
        body: RsxNodes,
    },
}

impl Parse for ControlFlow {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.peek(Token![if]) {
            input.parse::<Token![if]>()?;
            let cond: Expr = input.parse()?;
            let content;
            syn::braced!(content in input);
            let then_branch: RsxNodes = content.parse()?;
            let mut else_branch = None;
            if input.peek(Token![else]) {
                input.parse::<Token![else]>()?;
                if input.peek(Token![if]) {
                    return Err(input.error("Nested if else not yet supported in basic RSX parser"));
                }
                let content;
                syn::braced!(content in input);
                else_branch = Some(content.parse()?);
            }
            Ok(ControlFlow::If {
                cond,
                then_branch,
                else_branch,
            })
        } else if input.peek(Token![for]) {
            input.parse::<Token![for]>()?;
            let pat = syn::Pat::parse_multi_with_leading_vert(input)?;
            input.parse::<Token![in]>()?;
            let expr: Expr = input.parse()?;
            let content;
            syn::braced!(content in input);
            let body: RsxNodes = content.parse()?;
            Ok(ControlFlow::For { pat, expr, body })
        } else {
            Err(input.error("Expected if or for"))
        }
    }
}

impl ToTokens for ControlFlow {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        match self {
            ControlFlow::If {
                cond,
                then_branch,
                else_branch,
            } => {
                if let Some(else_b) = else_branch {
                    tokens.extend(quote! {
                        if #cond {
                            let mut __subnodes = #then_branch;
                            __nodes.extend(__subnodes);
                        } else {
                            let mut __subnodes = #else_b;
                            __nodes.extend(__subnodes);
                        }
                    });
                } else {
                    tokens.extend(quote! {
                        if #cond {
                            let mut __subnodes = #then_branch;
                            __nodes.extend(__subnodes);
                        }
                    });
                }
            }
            ControlFlow::For { pat, expr, body } => {
                tokens.extend(quote! {
                    for #pat in #expr {
                        let mut __subnodes = #body;
                        __nodes.extend(__subnodes);
                    }
                });
            }
        }
    }
}
