use crate::ast::*;
use std::collections::HashSet;
use syn::{
    Expr, Ident, LitStr, Result, Token, braced,
    ext::IdentExt,
    parse::{Parse, ParseStream},
};

impl Parse for RsxNodes {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut nodes = Vec::new();
        while !input.is_empty() {
            nodes.push(input.parse()?);
            // Optional comma separation
            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }
        Ok(RsxNodes { nodes })
    }
}

impl Parse for RsxNode {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.peek(Token![if]) || input.peek(Token![for]) {
            Ok(RsxNode::ControlFlow(input.parse()?))
        } else if input.peek(Token![<]) {
            // Basic Fragment syntax <> ... </>
            if input.peek2(Token![>]) {
                input.parse::<Token![<]>()?;
                input.parse::<Token![>]>()?;
                let content: RsxNodes = input.parse()?;
                input.parse::<Token![<]>()?;
                input.parse::<Token![/]>()?;
                input.parse::<Token![>]>()?;
                Ok(RsxNode::Fragment(content))
            } else {
                Err(input.error("Expected fragment or element"))
            }
        } else if input.peek(syn::token::Brace) {
            // { variable }
            let content;
            braced!(content in input);
            Ok(RsxNode::Text(LitStrOrExpr::Expr(content.parse()?)))
        } else if input.peek(LitStr) {
            Ok(RsxNode::Text(LitStrOrExpr::Lit(input.parse()?)))
        } else {
            // Ident check: Capitalized -> Component, lowercase -> Element
            let name: Ident = input.fork().parse()?;
            let first_char = name.to_string().chars().next().unwrap();
            if first_char.is_uppercase() {
                Ok(RsxNode::Component(input.parse()?))
            } else {
                Ok(RsxNode::Element(input.parse()?))
            }
        }
    }
}

impl Parse for Element {
    fn parse(input: ParseStream) -> Result<Self> {
        let name: Ident = input.parse()?;
        let span = name.span();
        let mut attributes = Vec::new();
        let mut children = Vec::new();

        let mut key = None;

        if input.peek(syn::token::Brace) {
            let content;
            braced!(content in input);
            while !content.is_empty() {
                // Heuristic:
                // if it looks like key: value, it's an attribute
                // if it looks like "string", { block }, or another element, it's a child.

                let fork = content.fork();
                if let Ok(ident) = fork.parse::<Ident>() {
                    if fork.peek(Token![:]) {
                        // Key: Value
                        let name = ident;
                        content.parse::<Ident>()?; // consume name
                        content.parse::<Token![:]>()?;

                        // Check if it is "key"
                        if name == "key" {
                            let val: Expr = if content.peek(LitStr) {
                                let lit: LitStr = content.parse()?;
                                syn::parse2(quote::quote! { #lit }).unwrap()
                            } else {
                                content.parse()?
                            };
                            key = Some(val);
                        } else {
                            // Normal attribute
                            let val = if content.peek(LitStr) {
                                AttributeValue::Lit(content.parse()?)
                            } else {
                                AttributeValue::Expr(content.parse()?)
                            };
                            attributes.push(Attribute { name, value: val });
                        }
                    } else if fork.is_empty() || fork.peek(Token![,]) {
                        // Shorthand
                        let name = ident;
                        content.parse::<Ident>()?; // consume
                        attributes.push(Attribute {
                            name,
                            value: AttributeValue::Shorthand,
                        });
                    } else {
                        // Child element
                        children.push(content.parse()?);
                    }
                } else if content.peek(LitStr)
                    || content.peek(syn::token::Brace)
                    || content.peek(Token![if])
                    || content.peek(Token![for])
                {
                    children.push(content.parse()?);
                } else {
                    children.push(content.parse()?);
                }

                if content.peek(Token![,]) {
                    content.parse::<Token![,]>()?;
                }
            }
        }

        Ok(Element {
            name,
            attributes,
            children,
            key,
            _span: span,
        })
    }
}

impl Parse for Component {
    fn parse(input: ParseStream) -> Result<Self> {
        let name: Ident = input.parse()?;
        let span = name.span();
        let mut props = Vec::new();
        let mut children = Vec::new(); // Support children injection later? 

        // Components accept Props via brace syntax: MyComp { prop: value }
        if input.peek(syn::token::Brace) {
            let content;
            braced!(content in input);
            while !content.is_empty() {
                // Components ONLY take props usually.
                // But if we support children, they need to be passed as a 'children' prop or special syntax.
                // Convention: If prop name is `children`, it's children.
                // Or we scan for props.

                // Strict Props: Ident : Value
                // Shorthand: Ident

                props.push(content.parse()?);

                if content.peek(Token![,]) {
                    content.parse::<Token![,]>()?;
                }
            }
        } else {
            // Allow parentheses for props? No, strict RSX usually braces.
            // Allow nothing -> No props.
        }

        Ok(Component {
            name,
            props,
            children,
            _span: span,
        })
    }
}

impl Parse for Attribute {
    fn parse(input: ParseStream) -> Result<Self> {
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
            Ok(Attribute {
                name,
                value: AttributeValue::Shorthand,
            })
        }
    }
}

impl Parse for Prop {
    fn parse(input: ParseStream) -> Result<Self> {
        let name: Ident = input.call(Ident::parse_any)?;
        if input.peek(Token![:]) {
            input.parse::<Token![:]>()?;
            Ok(Prop {
                name,
                value: PropValue::Expr(input.parse()?),
            })
        } else {
            Ok(Prop {
                name,
                value: PropValue::Shorthand,
            })
        }
    }
}

fn parse_until_brace(input: ParseStream) -> Result<Expr> {
    let mut tokens = proc_macro2::TokenStream::new();
    while !input.is_empty() {
        if input.peek(syn::token::Brace) {
            break;
        }
        tokens.extend(std::iter::once(input.parse::<proc_macro2::TokenTree>()?));
    }
    if tokens.is_empty() {
        return Err(input.error("Expected expression"));
    }
    syn::parse2(tokens)
}

impl Parse for ControlFlow {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.peek(Token![if]) {
            input.parse::<Token![if]>()?;
            // Custom parsing for condition to avoid greedy struct literal parsing
            let cond = parse_until_brace(input)?;

            let content;
            braced!(content in input);
            let then_branch: RsxNodes = content.parse()?;
            let mut else_branch = None;
            if input.peek(Token![else]) {
                input.parse::<Token![else]>()?;
                let content;
                braced!(content in input);
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

            // Custom parsing for iterator expr
            let expr = parse_until_brace(input)?;

            let content;
            braced!(content in input);
            let body: RsxNodes = content.parse()?;
            // Allow parsing key? for pat in expr key(k) { ... } or similar?
            // For now, assume key is derived or embedded.
            Ok(ControlFlow::For {
                pat,
                expr,
                body,
                key: None,
            })
        } else {
            Err(input.error("Expected if or for"))
        }
    }
}
