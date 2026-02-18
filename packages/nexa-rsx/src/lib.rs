use proc_macro::TokenStream;
use quote::ToTokens;
use syn::parse_macro_input;

mod ast;
mod codegen;
mod parse;

use ast::RsxNodes;

#[proc_macro]
pub fn rsx(input: TokenStream) -> TokenStream {
    let nodes = parse_macro_input!(input as RsxNodes);
    nodes.to_token_stream().into()
}
