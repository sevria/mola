use utoipa::openapi::{
    Deprecated, Ref, RefOr,
    content::Content,
    path::{HttpMethod, Operation, PathItem},
    request_body::RequestBody,
    response::{Response as OpenApiResponse, ResponseBuilder},
    schema::Schema,
};

pub use utoipa::openapi::{InfoBuilder, OpenApiBuilder, ServerBuilder};

// ---------------------------------------------------------------------------
// Scalar UI (embedded HTML – loads @scalar/api-reference from jsDelivr CDN)
// ---------------------------------------------------------------------------

/// Build the Scalar API reference HTML page with a dynamic title.
pub fn scalar_ui_html(title: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>{title}</title>
</head>
<body>
  <script id="api-reference" data-url="./openapi.json"></script>
  <script src="https://cdn.jsdelivr.net/npm/@scalar/api-reference"></script>
</body>
</html>"#
    )
}

// ---------------------------------------------------------------------------
// path – ergonomic helpers for building PathItem / Operation values
// ---------------------------------------------------------------------------

/// Re-export of utoipa's `HttpMethod` so callers can reference method kinds.
pub use utoipa::openapi::path::HttpMethod as Method;

/// Builder for an OpenAPI [`PathItem`].
///
/// # Example
///
/// ```ignore
/// use zenix::openapi::path::*;
///
/// let item = get("/users")
///     .tag("Users")
///     .description("List all users")
///     .json_response::<Vec<User>>("A list of users")
///     .build();
/// ```
pub mod path {
    use super::*;

    // -- Top-level helpers that return builders ----------------------------

    pub fn get(path: impl Into<String>) -> PathDoc {
        PathDoc::new(path, HttpMethod::Get)
    }
    pub fn post(path: impl Into<String>) -> PathDoc {
        PathDoc::new(path, HttpMethod::Post)
    }
    pub fn put(path: impl Into<String>) -> PathDoc {
        PathDoc::new(path, HttpMethod::Put)
    }
    pub fn delete(path: impl Into<String>) -> PathDoc {
        PathDoc::new(path, HttpMethod::Delete)
    }
    pub fn patch(path: impl Into<String>) -> PathDoc {
        PathDoc::new(path, HttpMethod::Patch)
    }

    // -- PathDoc builder ---------------------------------------------------

    /// Builder that collects metadata for a single HTTP method on a path and
    /// finally produces a [`PathItem`] (via [`.build()`](PathDoc::build)).
    pub struct PathDoc {
        path: String,
        method: HttpMethod,
        operation: Operation,
    }

    impl PathDoc {
        fn new(path: impl Into<String>, method: HttpMethod) -> Self {
            Self {
                path: path.into(),
                method,
                operation: Operation::new(),
            }
        }

        /// Attach an OpenAPI tag (e.g. `"Users"`).
        pub fn tag(mut self, tag: &str) -> Self {
            self.operation
                .tags
                .get_or_insert_with(Vec::new)
                .push(tag.to_string());
            self
        }

        /// Set the `summary` / short description.
        pub fn summary(mut self, summary: &str) -> Self {
            self.operation.summary = Some(summary.to_string());
            self
        }

        /// Set the `description` (longer form).
        pub fn description(mut self, desc: &str) -> Self {
            self.operation.description = Some(desc.to_string());
            self
        }

        /// Mark the operation as deprecated.
        pub fn deprecated(mut self) -> Self {
            self.operation.deprecated = Some(Deprecated::True);
            self
        }

        /// Add a JSON request body. Extracts the schema name from the type
        /// parameter `T` at runtime via `std::any::type_name`.
        pub fn json_request<T>(self, desc: &str) -> Self {
            let type_name = std::any::type_name::<T>();
            let schema_name = type_name.rsplit("::").next().unwrap_or(type_name);
            self.json_request_with_schema(desc, schema_name)
        }

        /// Add a JSON request body with an explicit schema name.
        pub fn json_request_with_schema(mut self, desc: &str, schema_name: &str) -> Self {
            let content = Content::new(Some(RefOr::Ref(Ref::from_schema_name(schema_name))));
            let mut request_body = RequestBody::new();
            request_body.description = Some(desc.to_string());
            request_body
                .content
                .insert("application/json".into(), content);
            self.operation.request_body = Some(request_body);
            self
        }

        /// Add a request body with the given description and content type
        /// (no schema reference — use `json_request_with_schema` for that).
        pub fn request_body(mut self, desc: &str, content_type: &str) -> Self {
            let mut request_body = RequestBody::new();
            request_body.description = Some(desc.to_string());
            request_body
                .content
                .insert(content_type.into(), Content::new(None::<RefOr<Schema>>));
            self.operation.request_body = Some(request_body);
            self
        }

        /// Add a JSON response for status 200. Extracts the schema name from
        /// the type parameter `T` at runtime.
        pub fn json_response<T>(self, desc: &str) -> Self {
            let type_name = std::any::type_name::<T>();
            let schema_name = type_name.rsplit("::").next().unwrap_or(type_name);
            self.json_content_response("200", desc, schema_name)
        }

        /// Add a JSON response with an explicit status code, description,
        /// and schema name.
        pub fn json_content_response(self, status: &str, desc: &str, schema_name: &str) -> Self {
            let content = Content::new(Some(RefOr::Ref(Ref::from_schema_name(schema_name))));
            let response = ResponseBuilder::new()
                .description(desc)
                .content("application/json", content)
                .build();
            self.response(status, response)
        }

        /// Add an arbitrary response by status code.
        pub fn response(mut self, status: &str, response: OpenApiResponse) -> Self {
            self.operation
                .responses
                .responses
                .insert(status.to_string(), RefOr::T(response));
            self
        }

        /// Consume the builder and return a `(path_string, PathItem)` pair
        /// ready to be registered with [`OpenApiDoc::path_item`].
        pub fn build(self) -> (String, PathItem) {
            let item = PathItem::new(self.method, self.operation);
            (self.path, item)
        }
    }

    impl From<PathDoc> for (String, PathItem) {
        fn from(doc: PathDoc) -> Self {
            doc.build()
        }
    }
}

// ---------------------------------------------------------------------------
// Convenience response builders
// ---------------------------------------------------------------------------

/// Create a `200 OK` JSON response with a description.
pub fn json_response(desc: &str) -> OpenApiResponse {
    ResponseBuilder::new().description(desc).build()
}

/// Create a response with a description.
pub fn response(desc: &str) -> OpenApiResponse {
    ResponseBuilder::new().description(desc).build()
}
