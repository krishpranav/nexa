use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput};

#[proc_macro_derive(Routable, attributes(route))]
pub fn routable_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;

    let variants = match input.data {
        Data::Enum(ref data) => &data.variants,
        _ => panic!("Routable can only be derived for enums"),
    };

    let mut match_arms = Vec::new();
    let mut display_arms = Vec::new();

    for variant in variants {
        let variant_name = &variant.ident;
        let mut route_path = None;

        for attr in &variant.attrs {
            if attr.path().is_ident("route") {
                // Parse #[route("path")]
                // Usually routing libs parse args.
                // Minimal implementation: expects string literal.
                let path: syn::LitStr = attr
                    .parse_args()
                    .expect("Route attribute expects a string literal path");
                route_path = Some(path.value());
            }
        }

        if let Some(path) = route_path {
            // For simple paths without params
            match_arms.push(quote! {
                #path => Some(Self::#variant_name),
            });
            display_arms.push(quote! {
                Self::#variant_name => write!(f, "{}", #path),
            });
        } else {
            // No route attribute? Error or skip?
            // panic!("All variants must have a #[route(...)] attribute");
            // Or maybe a 404 variant?
        }
    }

    let expanded = quote! {
        impl nexa_router::Routable for #name {
            fn from_path(path: &str) -> Option<Self> {
                match path {
                    #(#match_arms)*
                    _ => None,
                }
            }
        }

        impl std::fmt::Display for #name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                 match self {
                     #(#display_arms)*
                     _ => write!(f, "/unknown"),
                 }
            }
        }
    };

    TokenStream::from(expanded)
}
