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
pub fn camunda_connector(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as ConnectorArgs);
    let name = args.name;
    let operation = args.operation;

    let input_fn = parse_macro_input!(item as ItemFn);
    let fn_name = &input_fn.sig.ident;

    if input_fn.sig.inputs.len() != 2 {
        return Error::new_spanned(&input_fn.sig.inputs, "Expected exactly 2 parameters: (id: u64, params: T)")
            .to_compile_error().into();
    }
    if input_fn.sig.asyncness.is_none() {
        return Error::new_spanned(&input_fn.sig.fn_token, "Function must be async")
            .to_compile_error().into();
    }
    let params_arg = input_fn.sig.inputs.iter().nth(1).unwrap();
    let input_ty = if let FnArg::Typed(pt) = params_arg {
        &pt.ty
    } else {
        return Error::new_spanned(params_arg, "Expected typed second param").to_compile_error().into();
    };

    let params_struct = format_ident!("Params{}{}", capitalize_first(&name), capitalize_first(&operation));
    let request_struct = format_ident!("Request{}{}", capitalize_first(&name), capitalize_first(&operation));
    let exec_fn = format_ident!("exec_raw_{}_{}", &name, &operation);

    let out = quote! {
        #[derive(Debug, serde::Deserialize)]
        pub struct #params_struct {
            pub operation: String,
            pub input: #input_ty,
        }

        #[derive(Debug, serde::Deserialize)]
        pub struct #request_struct {
            pub id: u64,
            pub params: #params_struct,
        }

        #input_fn

        fn #exec_fn(bytes: axum::body::Bytes) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<serde_json::Value, String>> + Send + 'static>> {
            Box::pin(async move {
                // Full, typed deserialization for THIS connector/op
                let req: #request_struct = serde_json::from_slice(&bytes)
                    .map_err(|e| format!("Bad JSON for `{}`/`{}`: {}", #name, #operation, e))?;

                // (Optional) sanity check — not strictly needed since dispatcher already matched
                if req.params.operation != #operation {
                    return Err(format!("Operation mismatch: expected `{}`, got `{}`", #operation, req.params.operation));
                }

                // Call user's handler
                match #fn_name(req.id, req.params.input).await {
                    Ok(out) => serde_json::to_value(out).map_err(|e| e.to_string()),
                    Err(e) => Err(e.to_string()),
                }
            })
        }

        ::inventory::submit! {
            crate::connectors::ConnectorRecipe {
                name: #name,
                operation: #operation,
                exec_raw: #exec_fn,
            }
        }
    };
    out.into()
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
        mod connectors {

            pub struct ConnectorRecipe {
                pub name: &'static str,
                pub operation: &'static str,
                pub exec_raw: fn(axum::body::Bytes) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<serde_json::Value, String>> + Send + 'static>>,
            }

            ::inventory::collect!(ConnectorRecipe);
        }

        #[derive(Deserialize)]
        struct OpPeek {
            params: OpPeekParams,
        }
        #[derive(Deserialize)]
        struct OpPeekParams {
            operation: String,
        }

        fn build_table() -> std::collections::HashMap<(String, String), fn(axum::body::Bytes) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<serde_json::Value, String>> + Send + 'static>>> {
            let mut table = std::collections::HashMap::new();
            for r in ::inventory::iter::<crate::connectors::ConnectorRecipe> {
                table.insert((r.name.to_string(), r.operation.to_string()), r.exec_raw);
            }
            table
        }

        async fn dispatch(
            axum::extract::Path(name): axum::extract::Path<String>,
            body: axum::body::Bytes,
            ) -> Result<axum::Json<serde_json::Value>, (axum::http::StatusCode, String)> {
            // 1) Peek op
            let peek: OpPeek = serde_json::from_slice(&body)
                .map_err(|_| (axum::http::StatusCode::BAD_REQUEST, "Invalid JSON envelope".to_string()))?;

            static ONCE: std::sync::OnceLock<std::collections::HashMap<(String, String), fn(axum::body::Bytes) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<serde_json::Value, String>> + Send + 'static>>>> =
                std::sync::OnceLock::new();
            let table = ONCE.get_or_init(build_table);

            let key = (name, peek.params.operation.clone());
            let exec = table
                .get(&key)
                .ok_or_else(|| (axum::http::StatusCode::BAD_REQUEST, format!("Unsupported operation `{}`", peek.params.operation)))?;

            match exec(body).await {
                Ok(val) => Ok(axum::Json(val)),
                Err(e) => Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e)),
            }
        }

        #[tokio::main]
        async fn main() {
            let app = axum::Router::new().route("/csp/{name}", axum::routing::post(dispatch));
            let addr = ::std::net::SocketAddr::from(([0,0,0,0], #port));
            let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
            println!("🚀 Listening on {addr}");
            axum::serve(listener, app.into_make_service()).await.unwrap();
        }
    }.into()
}
