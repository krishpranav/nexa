use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input, Result, Token,
};

#[proc_macro]
pub fn rsx(input: TokenStream) -> TokenStream {
    let call = parse_macro_input!(input as RsxCall);
    call.to_token_stream().into()
}

pub struct RsxCall {
    pub root: RsxNode,
}

impl Parse for RsxCall {
    fn parse(input: ParseStream) -> Result<Self> {
        // For simplicity, rsx! takes a single root node for now, or a list which becomes a Fragment?
        // Prompt says `rsx! { div { ... } }`.
        Ok(RsxCall {
            root: input.parse()?,
        })
    }
}

pub enum RsxNode {
    Element(Element),
    Text(syn::LitStr),
    // Block(syn::Block), // expressions like { foo }
}

impl Parse for RsxNode {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.peek(syn::LitStr) {
            Ok(RsxNode::Text(input.parse()?))
        } else if input.peek(syn::Ident) {
            Ok(RsxNode::Element(input.parse()?))
        } else {
            Err(input.error("Expected element or string literal"))
        }
    }
}

impl ToTokens for RsxNode {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        match self {
            RsxNode::Element(el) => el.to_tokens(tokens),
            RsxNode::Text(txt) => {
                let s = txt.value();
                tokens.extend(quote! {
                    nexa_core::get_active_arena(|arena| {
                        arena.insert(nexa_core::VirtualNode::Text(nexa_core::Text {
                            text: #s.to_string(),
                            parent: None,
                        }))
                    })
                });
            }
        }
    }
}

pub struct Element {
    pub name: syn::Ident,
    pub children: Vec<RsxNode>,
    // attributes...
}

impl Parse for Element {
    fn parse(input: ParseStream) -> Result<Self> {
        let name: syn::Ident = input.parse()?;
        let content;
        syn::braced!(content in input);

        let mut children = Vec::new();
        while !content.is_empty() {
            children.push(content.parse()?);
            // Optional comma ?
            if content.peek(Token![,]) {
                let _: Token![,] = content.parse()?;
            }
        }

        Ok(Element { name, children })
    }
}

impl ToTokens for Element {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let name_str = self.name.to_string();
        let children = &self.children;

        tokens.extend(quote! {
            nexa_core::get_active_arena(|arena| {
                let children_ids: smallvec::SmallVec<[nexa_core::NodeId; 4]> = smallvec::smallvec![
                    #(#children),*
                ];

                arena.insert(nexa_core::VirtualNode::Element(nexa_core::Element {
                    tag: #name_str,
                    props: smallvec::SmallVec::new(), // Attributes not fully parsed yet
                    children: children_ids,
                    parent: None,
                }))
            })
        });
    }
}
