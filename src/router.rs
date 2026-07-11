use anyhow::Result;
use axum::routing::MethodRouter;
use axum::{Json, Router as AxumRouter, response::Html, routing::get};
use log::info;
use serde::Serialize;
use tokio::net::TcpListener;
use utoipa::openapi::schema::{Components, Schema};
use utoipa::openapi::{OpenApi, RefOr};

use crate::openapi::scalar_ui_html;
use utoipa::openapi::path::PathItem;

/// Opaque handle produced by the [`#[endpoint]`](macro@crate::endpoint)
/// attribute macro. Pass it to [`Router::add`] to register the handler and
/// its OpenAPI metadata.
pub struct RouteHandle {
    register: fn(&mut Router),
}

impl RouteHandle {
    /// Create a new handle from a registration function. This is called by the
    /// `#[endpoint]` proc macro; you should not need to construct this manually.
    #[doc(hidden)]
    pub const fn new(register: fn(&mut Router)) -> Self {
        Self { register }
    }
}

pub struct Router {
    router: AxumRouter,
    openapi: Option<OpenApi>,
}

impl Router {
    pub fn new() -> Self {
        Self {
            router: AxumRouter::new(),
            openapi: None,
        }
    }

    /// Enable OpenAPI documentation generation. The spec will be served at
    /// `/openapi.json` and a Scalar API reference at `/docs` when the server runs.
    pub fn with_openapi(mut self, openapi: OpenApi) -> Self {
        self.openapi = Some(openapi);
        self
    }

    /// Register OpenAPI documentation for a path. Accepts anything that can
    /// be converted into a `(path_string, `[`PathItem`]`)` pair.
    ///
    /// The easiest way to build the argument is with the helpers in
    /// [`crate::openapi::path`], e.g.:
    ///
    /// ```ignore
    /// server.describe(path::get("/users")
    ///     .tag("Users")
    ///     .description("List all users")
    ///     .json_response::<Vec<User>>("A list of users"));
    /// ```
    pub fn describe(&mut self, item: impl Into<(String, PathItem)>) -> &mut Self {
        let openapi = self.openapi.get_or_insert_with(OpenApi::default);
        let (path_str, path_item) = item.into();
        openapi.paths.paths.insert(path_str, path_item);
        self
    }

    /// Register a JSON Schema in the OpenAPI `components/schemas` section.
    /// The `schema_json` value is deserialized into a [`utoipa::openapi::schema::Schema`].
    pub fn add_schema(&mut self, name: &str, schema_json: serde_json::Value) -> &mut Self {
        if let Ok(schema) = serde_json::from_value::<Schema>(schema_json) {
            let openapi = self.openapi.get_or_insert_with(OpenApi::default);
            let components = openapi.components.get_or_insert_with(Components::new);
            components
                .schemas
                .insert(name.to_string(), RefOr::T(schema));
        }
        self
    }

    /// Register a [`#[endpoint]`](macro@crate::endpoint)-annotated handler.
    /// Use with the [`handler!`](macro@crate::handler) macro to reference the
    /// generated handle.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let server = Router::new()
    ///     .with_openapi(openapi)
    ///     .add(handler!(hello));
    /// ```
    #[allow(clippy::should_implement_trait)]
    pub fn add(mut self, handle: RouteHandle) -> Self {
        (handle.register)(&mut self);
        self
    }

    /// Nest another [`Router`] under a path prefix. All routes from the nested
    /// router are served under the given prefix, and their OpenAPI paths are
    /// merged accordingly.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let email_router = Router::new().add(handler!(send_email));
    /// let main_router = Router::new()
    ///     .with_openapi(openapi)
    ///     .nest("/emails", email_router);
    /// ```
    pub fn nest(mut self, prefix: &str, nested: Router) -> Self {
        // Merge OpenAPI paths from the nested router, prefixing each path
        // NOTE: must access nested.openapi BEFORE nested.router is moved
        if let Some(nested_openapi) = nested.openapi {
            let main_openapi = self.openapi.get_or_insert_with(OpenApi::default);
            for (path, item) in nested_openapi.paths.paths {
                let prefixed = if path == "/" {
                    prefix.to_string()
                } else {
                    format!("{prefix}{path}")
                };
                main_openapi.paths.paths.insert(prefixed, item);
            }
            // Merge schemas from nested router
            if let Some(nested_components) = nested_openapi.components {
                let main_components = main_openapi
                    .components
                    .get_or_insert_with(utoipa::openapi::schema::Components::new);
                for (name, schema) in nested_components.schemas {
                    main_components.schemas.insert(name, schema);
                }
            }
        }

        // Merge the axum routers (consume nested.router last)
        self.router = self.router.nest(prefix, nested.router);

        self
    }

    /// Add a route with a custom [`MethodRouter`] (e.g. from `axum::routing::get().post()`).
    /// Use this for advanced handlers that need extractors (state, path params, etc.).
    pub fn add_route(&mut self, path: &str, method_router: MethodRouter) -> &mut Self {
        let router = std::mem::take(&mut self.router);
        self.router = router.route(path, method_router);
        self
    }

    /// Register a GET route. The handler must be an async function (or closure) that
    /// returns a serializable value — it will be automatically wrapped in `Json`.
    pub fn get<F, Fut, T>(&mut self, path: &str, handler: F) -> &mut Self
    where
        F: Fn() -> Fut + Clone + Send + Sync + 'static,
        Fut: std::future::Future<Output = T> + Send,
        T: Serialize + 'static,
    {
        self.add_route(path, axum::routing::get(wrap_json_handler(handler)))
    }

    /// Register a POST route. See [`Self::get`] for handler expectations.
    pub fn post<F, Fut, T>(&mut self, path: &str, handler: F) -> &mut Self
    where
        F: Fn() -> Fut + Clone + Send + Sync + 'static,
        Fut: std::future::Future<Output = T> + Send,
        T: Serialize + 'static,
    {
        self.add_route(path, axum::routing::post(wrap_json_handler(handler)))
    }

    /// Register a PUT route. See [`Self::get`] for handler expectations.
    pub fn put<F, Fut, T>(&mut self, path: &str, handler: F) -> &mut Self
    where
        F: Fn() -> Fut + Clone + Send + Sync + 'static,
        Fut: std::future::Future<Output = T> + Send,
        T: Serialize + 'static,
    {
        self.add_route(path, axum::routing::put(wrap_json_handler(handler)))
    }

    /// Register a DELETE route. See [`Self::get`] for handler expectations.
    pub fn delete<F, Fut, T>(&mut self, path: &str, handler: F) -> &mut Self
    where
        F: Fn() -> Fut + Clone + Send + Sync + 'static,
        Fut: std::future::Future<Output = T> + Send,
        T: Serialize + 'static,
    {
        self.add_route(path, axum::routing::delete(wrap_json_handler(handler)))
    }

    /// Register a PATCH route. See [`Self::get`] for handler expectations.
    pub fn patch<F, Fut, T>(&mut self, path: &str, handler: F) -> &mut Self
    where
        F: Fn() -> Fut + Clone + Send + Sync + 'static,
        Fut: std::future::Future<Output = T> + Send,
        T: Serialize + 'static,
    {
        self.add_route(path, axum::routing::patch(wrap_json_handler(handler)))
    }

    pub async fn run(self, address: &str) -> Result<()> {
        let listener = TcpListener::bind(address).await?;

        info!("Running HTTP router on {}", address);

        let router = if let Some(openapi) = self.openapi {
            let spec_json = serde_json::to_value(&openapi).unwrap_or_default();
            let title = format!("{} API Docs", openapi.info.title);
            let ui_html = scalar_ui_html(&title);

            self.router
                .route(
                    "/openapi.json",
                    get(move || {
                        let json = spec_json.clone();
                        async move { Json(json) }
                    }),
                )
                .route("/", get(move || async move { Html(ui_html) }))
        } else {
            self.router
        };

        axum::serve(listener, router).await?;

        Ok(())
    }
}

/// Wraps a handler `Fn() -> Fut` whose output is `Serialize` into an axum-compatible
/// handler that automatically wraps the return value in [`Json`].
fn wrap_json_handler<F, Fut, T>(handler: F) -> impl axum::handler::Handler<((),), ()>
where
    F: Fn() -> Fut + Clone + Send + Sync + 'static,
    Fut: std::future::Future<Output = T> + Send,
    T: Serialize + 'static,
{
    move || async move { Json(handler().await) }
}
