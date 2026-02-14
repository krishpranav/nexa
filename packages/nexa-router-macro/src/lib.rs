use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, parse_macro_input};

#[proc_macro_derive(Routable, attributes(route))]
pub fn routable_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;

    let variants = match input.data {
        Data::Enum(ref data) => &data.variants,
        _ => panic!("Routable can only be derived for enums"),
    };

    let mut from_path_arms = Vec::new();
    let mut to_string_arms = Vec::new();

    for variant in variants {
        let variant_name = &variant.ident;
        let mut route_path = None;

        for attr in &variant.attrs {
            if attr.path().is_ident("route") {
                let path: syn::LitStr = attr
                    .parse_args()
                    .expect("Route attribute expects a string literal");
                route_path = Some(path.value());
            }
        }

        let path = route_path.expect("All variants must have a #[route(...)] attribute");
        let segments_count = path.split('/').filter(|s| !s.is_empty()).count();
        let segment_strings: Vec<String> = path
            .split('/')
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();

        // Handle variants with fields (dynamic segments)
        match &variant.fields {
            syn::Fields::Unit => {
                from_path_arms.push(quote! {
                    #path => Some(Self::#variant_name),
                });
                to_string_arms.push(quote! {
                    Self::#variant_name => write!(f, "{}", #path),
                });
            }
            syn::Fields::Named(fields) => {
                let field_idents: Vec<_> = fields.named.iter().map(|f| &f.ident).collect();

                // Logic to parse segments and extract params
                from_path_arms.push(quote! {
                    p if {
                        let p_segs: Vec<&str> = p.split('/').filter(|s| !s.is_empty()).collect();
                        if p_segs.len() == #segments_count {
                             let mut matches = true;
                             let template_segs = vec![#(#segment_strings),*];
                             for (i, t_seg) in template_segs.iter().enumerate() {
                                 if !t_seg.starts_with(':') && t_seg != &p_segs[i] {
                                     matches = false;
                                     break;
                                 }
                             }
                             matches
                        } else { false }
                    } => {
                        let p_segs: Vec<&str> = p.split('/').filter(|s| !s.is_empty()).collect();
                        Some(Self::#variant_name { #(#field_idents: p_segs[0].to_string()),* })
                    }
                });

                to_string_arms.push(quote! {
                    Self::#variant_name { #(#field_idents),* } => {
                        let mut p = #path.to_string();
                        #( p = p.replace(&format!(":{}", stringify!(#field_idents)), #field_idents); )*
                        write!(f, "{}", p)
                    }
                });
            }
            _ => panic!("Only unit and named fields supported for Routable"),
        }
    }

    let expanded = quote! {
        impl nexa_router::Routable for #name {
            fn from_path(path: &str) -> Option<Self> {
                let path = path.split('?').next().unwrap_or("/");
                match path {
                    #(#from_path_arms)*
                    _ => None,
                }
            }
        }

        impl std::fmt::Display for #name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    #(#to_string_arms)*
                }
            }
        }
    };

    TokenStream::from(expanded)
}
