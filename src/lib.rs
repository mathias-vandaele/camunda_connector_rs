use proc_macro::TokenStream;

use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::{
    Error, FnArg, ItemFn, LitInt, Token, parse_macro_input,
};

struct ConnectorArgs {
    name: String,
    operation: String,
}
// Abridged Parse impl for brevity
impl Parse for ConnectorArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut path = None;
        let mut operation = None;
        while !input.is_empty() {
            let key: syn::Ident = input.parse()?;
            input.parse::<Token![=]>()?;
            if key == "name" {
                let value: syn::LitStr = input.parse()?;
                path = Some(value.value());
            } else if key == "operation" {
                let value: syn::LitStr = input.parse()?;
                operation = Some(value.value());
            }else {
                return Err(Error::new_spanned(key, "Unknown attribute key"));
            }
            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }
        Ok(ConnectorArgs {
            name: path.ok_or_else(|| syn::Error::new(input.span(), "Missing 'path' parameter"))?,
            operation: operation.ok_or_else(|| syn::Error::new(input.span(), "Missing 'operation' parameter"))?,
        })
    }
}


fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

#[proc_macro_attribute]
pub fn camunda_connector(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(_attr as ConnectorArgs);
    let name = args.name.as_str();
    let operation = args.operation.as_str();
    let full_path = format!("/csp/{}/{}", name, operation);
    let connector_request_struct = format_ident!("ConnectorRequest{}{}", capitalize_first(name), capitalize_first(&operation));
    let input_fn = parse_macro_input!(item as ItemFn);
    let fn_name = &input_fn.sig.ident;

    let builder_fn_name = format_ident!("_create_{}_router_builder", fn_name);

    if input_fn.sig.inputs.len() != 2 {
        return Error::new_spanned(&input_fn.sig.inputs, "Expected exactly 2 parameters: id: Option<u64>, params: T").to_compile_error().into();
    }
    if input_fn.sig.asyncness.is_none() {
        return Error::new_spanned(&input_fn.sig.fn_token, "Function must be async").to_compile_error().into();
    }

    // Get the type of the third parameter, which holds the business data.
    let params_arg = input_fn.sig.inputs.iter().nth(1).unwrap();
    let input_ty = if let FnArg::Typed(pt) = params_arg {
        &pt.ty
    } else {
        panic!("Expected typed arg");
    };

    let generated_code = quote! {

        #[derive(Debug, serde::Deserialize)]
        pub struct #connector_request_struct {
            pub id: u64,
            pub params: #input_ty,
        }

        #input_fn

        fn #builder_fn_name() -> axum::Router {
            async fn handler(axum::Json(payload): axum::Json<#connector_request_struct>) -> impl axum::response::IntoResponse {
                match #fn_name(payload.id, payload.params).await {
                    Ok(data) => axum::response::IntoResponse::into_response(axum::Json(data)),
                    Err(e) => axum::response::IntoResponse::into_response((axum::http::StatusCode::INTERNAL_SERVER_ERROR, format!("{}", e))),
                }
            }
            axum::Router::new().route(#full_path, axum::routing::post(handler))
        }

        ::inventory::submit! {
            crate::connectors::ConnectorRecipe {
                name : #name,
                operation : #operation,
                create_router: #builder_fn_name,
            }
        }
    };

    generated_code.into()
}


struct MainArgs {
    port: LitInt,
}

impl Parse for MainArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let key: syn::Ident = input.parse()?;
        if key != "port" {
            return Err(Error::new_spanned(key, "Expected `port`"));
        }
        input.parse::<Token![=]>()?;
        let port: LitInt = input.parse()?;
        Ok(MainArgs { port })
    }
}

#[proc_macro]
pub fn connector_main(attr: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as MainArgs);
    let port = &args.port;

    quote! {
        use axum::Router;
        // This macro expands to the full main function
        mod connectors {
            use axum::Router;
            pub struct ConnectorRecipe {
                pub name: &'static str,
                pub operation: &'static str,
                pub create_router: fn() -> Router,
            }

            ::inventory::collect!(ConnectorRecipe);
        }

        #[tokio::main]
        async fn main() {
            let mut app = axum::Router::new();

            // The loop iterates over the inventory populated by the `#[connector]` macros
            for recipe in ::inventory::iter::<crate::connectors::ConnectorRecipe> {
                println!("Merging router for recipe: name => {} | operation => {}", recipe.name, recipe.operation);
                app = app.merge((recipe.create_router)());
            }

            let addr = ::std::net::SocketAddr::from(([0, 0, 0, 0], #port));
            println!("🚀 Connector server listening on {}", addr);

            // Updated for axum 0.7+: Use axum::serve with a Tokio TCP listener.
            let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
            axum::serve(listener, app.into_make_service()).await.unwrap();
        }
    }.into()
}
