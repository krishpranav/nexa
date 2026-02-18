use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use crate::ast::*;

impl ToTokens for RsxNodes {
    fn to_tokens(&self, tokens: &mut TokenStream) {
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

impl ToTokens for RsxNode {
    fn to_tokens(&self, tokens: &mut TokenStream) {
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

impl ToTokens for Element {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let tag = self.name.to_string();
        let children = &self.children;
        let mut props = Vec::new();
        let mut listeners = Vec::new();

        for attr in &self.attributes {
            let name_str = attr.name.to_string();
            if name_str.starts_with("on") {
                let event_name = name_str.trim_start_matches("on").to_lowercase();
                let val = match &attr.value {
                    AttributeValue::Lit(_) => quote! { panic!("Lit events not supported") },
                    AttributeValue::Expr(e) => quote! { #e },
                    AttributeValue::Shorthand => {
                         let n = &attr.name;
                         quote! { #n }
                    }
                };
                listeners.push(quote! {
                    nexa_core::vdom::EventListener {
                        name: #event_name,
                        cb: std::rc::Rc::new(std::cell::RefCell::new(#val)),
                    }
                });
            } else {
                let val = match &attr.value {
                    AttributeValue::Lit(l) => {
                        let s = l.value();
                        quote! { #s.to_string() }
                    }
                    AttributeValue::Expr(e) => quote! { format!("{}", #e) },
                    AttributeValue::Shorthand => {
                         let n = &attr.name;
                         quote! { format!("{}", #n) }
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
                // Generate children
                let mut __el_nodes: smallvec::SmallVec<[nexa_core::NodeId; 4]> = smallvec::SmallVec::new();
                {
                    let mut __nodes = __el_nodes; // Shadow __nodes for children context
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

impl ToTokens for Component {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let name = &self.name;
        let props_name = quote::format_ident!("{}Props", name);
        
        // Generate props struct init
        let mut fields = Vec::new();
        for prop in &self.props {
            let field_name = &prop.name;
            match &prop.value {
                PropValue::Expr(e) => {
                    fields.push(quote! { #field_name: #e });
                }
                PropValue::Shorthand => {
                    fields.push(quote! { #field_name: #field_name });
                }
            }
        }
        
        // Components are functions taking props and returning NodeId
        tokens.extend(quote! {
            __nodes.push(#name(#props_name {
                #(#fields),*
            }));
        });
    }
}

impl ToTokens for ControlFlow {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            ControlFlow::If { cond, then_branch, else_branch } => {
                let else_block = if let Some(else_b) = else_branch {
                    quote! { else {
                        let mut __subnodes = #else_b;
                        __nodes.extend(__subnodes);
                    }}
                } else {
                    quote! {}
                };
                
                tokens.extend(quote! {
                    if #cond {
                        let mut __subnodes = #then_branch;
                        __nodes.extend(__subnodes);
                    } #else_block
                });
            }
            ControlFlow::For { pat, expr, body, key: _ } => {
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
