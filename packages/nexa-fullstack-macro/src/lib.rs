use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, FnArg, ItemFn, Pat};

#[proc_macro_attribute]
pub fn server(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(item as ItemFn);
    let fn_name = &input_fn.sig.ident;
    let fn_name_str = fn_name.to_string();
    let inputs = &input_fn.sig.inputs;
    let output = &input_fn.sig.output;
    let block = &input_fn.block;
    let vis = &input_fn.vis;
    let api_path = format!("/api/{}", fn_name_str);

    // Collect arg names and types for struct generation
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

    // Server Side Implementation
    // We register the function logic.
    // Ideally we use a distributed slice or ctor, but for now we just generate the function normal body.
    // The user has to manually register? Or we use `inventory`?
    // "Minimal Axum integration" -> let's assume we just generate the function.
    // But we need to handle the untyped JSON execution for the generic router.

    // Valid for "ssr" feature
    let server_impl = quote! {
        #[cfg(feature = "ssr")]
        #vis async fn #fn_name(#inputs) #output {
            #block
        }

        // Generate a registration helper that the user can call to register this fn
        #[cfg(feature = "ssr")]
        pub mod #fn_name {
            use super::*;
            use nexa_fullstack::server::{register_server_fn, ServerFnError};

            #[derive(serde::Deserialize)]
            struct Args {
                #(#arg_names: #arg_types),*
            }

            pub fn register() {
                // Use std::sync::Arc::new instead of Box::new to match ServerFnHandler type
                register_server_fn(#fn_name_str, std::sync::Arc::new(|json_args| Box::pin(async move {
                    let args: Args = serde_json::from_value(json_args)
                        .map_err(|e| ServerFnError { message: e.to_string() })?;

                    let res = super::#fn_name(#(args.#arg_names),*).await;

                    // Assume res is Result<T, E> or just T?
                    // For now, lets assume Result<T, ServerFnError> as required by runtime
                    // We need to verify return type compatibility or wrap it.
                    // This is tricky without strict type inspection.
                    // Let's assume serialization of success:

                    match res {
                        Ok(val) => {
                            let json = serde_json::to_value(val)
                                .map_err(|e| ServerFnError { message: e.to_string() })?;
                            Ok(json)
                        }
                        Err(e) => Err(ServerFnError { message: e.to_string() }), // Stringify error
                    }
                })));
            }
        }
    };

    // Client Side Implementation
    // Valid for not "ssr" (wasm)

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

            nexa_fullstack::client::call_server_fn(#api_path, args).await
        }
    };

    let expanded = quote! {
        #server_impl
        #client_impl
    };

    TokenStream::from(expanded)
}
