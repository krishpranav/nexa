use proc_macro::TokenStream;
use quote::quote;
use syn::{FnArg, ItemFn, Pat, parse_macro_input};

#[proc_macro_attribute]
pub fn server(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(item as ItemFn);

    // Minimal attribute parsing for encoding
    let mut use_cbor = false;
    let attr_str = attr.to_string();
    if attr_str.contains("encoding = \"cbor\"") || attr_str.contains("encoding=\"cbor\"") {
        use_cbor = true;
    }

    let fn_name = &input_fn.sig.ident;
    let fn_name_str = fn_name.to_string();
    let inputs = &input_fn.sig.inputs;
    let output = &input_fn.sig.output;
    let block = &input_fn.block;
    let vis = &input_fn.vis;
    let api_path = format!("/api/{}", fn_name_str);

    let mut arg_names = Vec::new();
    let mut arg_types = Vec::new();

    for arg in inputs {
        if let FnArg::Typed(pat_type) = arg {
            if let Pat::Ident(pat_ident) = &*pat_type.pat {
                arg_names.push(&pat_ident.ident);
                arg_types.push(&pat_type.ty);
            }
        }
    }

    let server_impl = quote! {
        #[cfg(feature = "ssr")]
        #vis async fn #fn_name(#inputs) #output {
            #block
        }

        #[cfg(feature = "ssr")]
        pub mod #fn_name {
            use super::*;
            use nexa_fullstack::server::{register_server_fn, ServerFnError};

            #[derive(serde::Deserialize)]
            struct Args {
                #(#arg_names: #arg_types),*
            }

            pub fn register() {
                register_server_fn(#fn_name_str, std::sync::Arc::new(|json_args| Box::pin(async move {
                    let args: Args = serde_json::from_value(json_args)
                        .map_err(|e| ServerFnError { message: e.to_string() })?;

                    let res = super::#fn_name(#(args.#arg_names),*).await;
                    match res {
                        Ok(val) => {
                            let json = serde_json::to_value(val)
                                .map_err(|e| ServerFnError { message: e.to_string() })?;
                            Ok(json)
                        }
                        Err(e) => Err(ServerFnError { message: e.to_string() }),
                    }
                })));
            }
        }
    };

    let client_impl = quote! {
        #[cfg(not(feature = "ssr"))]
        #vis async fn #fn_name(#inputs) #output {
            #[derive(serde::Serialize)]
            struct Args<'a> {
                #(#arg_names: &'a #arg_types),*
            }

            let args = Args {
                #(#arg_names: &#arg_names),*
            };

            nexa_fullstack::client::wasm::call_server_fn(#api_path, args, #use_cbor).await
        }
    };

    TokenStream::from(quote! {
        #server_impl
        #client_impl
    })
}
