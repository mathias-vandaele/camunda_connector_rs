use proc_macro::TokenStream;

use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::{
    Error, FnArg, ItemFn, LitInt, Token, parse_macro_input,
};

struct ConnectorArgs {
    path: String,
}
// Abridged Parse impl for brevity
impl Parse for ConnectorArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut path = None;
        while !input.is_empty() {
            let key: syn::Ident = input.parse()?;
            input.parse::<Token![=]>()?;
            if key == "path" {
                let value: syn::LitStr = input.parse()?;
                path = Some(value.value());
            } else {
                return Err(Error::new_spanned(key, "Unknown attribute key"));
            }
            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }
        Ok(ConnectorArgs {
            path: path.ok_or_else(|| syn::Error::new(input.span(), "Missing 'path' parameter"))?,
        })
    }
}


#[proc_macro_attribute]
pub fn camunda_connector(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(_attr as ConnectorArgs);
    let path = args.path.as_str();
    let full_path = format!("/csp/{}", path);
    let input_fn = parse_macro_input!(item as ItemFn);
    let fn_name = &input_fn.sig.ident;

    let builder_fn_name = format_ident!("_create_{}_router_builder", fn_name);

    if input_fn.sig.inputs.len() != 3 {
        return Error::new_spanned(&input_fn.sig.inputs, "Expected exactly 3 parameters: id: Option<u64>, method: String, params: T").to_compile_error().into();
    }
    if input_fn.sig.asyncness.is_none() {
        return Error::new_spanned(&input_fn.sig.fn_token, "Function must be async").to_compile_error().into();
    }

    // Get the type of the third parameter, which holds the business data.
    let params_arg = input_fn.sig.inputs.iter().nth(2).unwrap();
    let input_ty = if let FnArg::Typed(pt) = params_arg {
        &pt.ty
    } else {
        panic!("Expected typed arg");
    };

    let generated_code = quote! {

        use axum::response::IntoResponse;

        #[derive(Debug, serde::Deserialize)]
        pub struct ConnectorRequest {
            pub id: u64,
            pub method: String,
            pub params: #input_ty,
        }

        #input_fn

        fn #builder_fn_name() -> axum::Router {
            async fn handler(axum::Json(payload): axum::Json<ConnectorRequest>) -> impl axum::response::IntoResponse {
                match #fn_name(payload.id, payload.method, payload.params).await {
                    Ok(data) => axum::Json(data).into_response(),
                    Err(e) => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, format!("{}", e)).into_response(),
                }
            }
            axum::Router::new().route(#full_path, axum::routing::post(handler))
        }

        // 3. Use `inventory::submit!` to create a recipe and register it in the global collection.
        // This code runs when the program starts up.
        ::inventory::submit! {
            crate::connectors::ConnectorRecipe {
                name : #path,
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
                pub create_router: fn() -> Router,
            }

            ::inventory::collect!(ConnectorRecipe);
        }

        #[tokio::main]
        async fn main() {
            let mut app = axum::Router::new();

            // The loop iterates over the inventory populated by the `#[connector]` macros
            for recipe in ::inventory::iter::<crate::connectors::ConnectorRecipe> {
                println!("Merging router for recipe: {}", recipe.name);
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
