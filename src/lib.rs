pub mod openapi;
pub mod router;

pub use router::{RouteHandle, Router};

// Re-export the #[endpoint] attribute macro so users can write
// `#[mola::endpoint(path = "...", ...)]`.
pub use mola_macros::endpoint;

// Re-export paste for use by the `routes!` macro expansion.
#[doc(hidden)]
pub use paste::paste;

/// Expands a [`#[endpoint]`](macro@crate::endpoint)-annotated handler name into
/// its generated [`RouteHandle`] const. Use with [`Router::add`].
///
/// # Example
///
/// ```ignore
/// let router = Router::new()
///     .with_openapi(openapi)
///     .add(handler!(hello));
/// ```
#[macro_export]
macro_rules! handler {
    ($handler:ident) => {
        $crate::paste! { [< $handler _handle >] }
    };
}

/// Convenience macro that registers one or more [`#[endpoint]`](macro@crate::endpoint)-annotated
/// handlers on a [`Router`].
///
/// For builder-pattern chaining, prefer [`Router::add`] with [`handler!`].
///
/// # Example
///
/// ```ignore
/// // Chained style (preferred):
/// let router = Router::new()
///     .with_openapi(openapi)
///     .add(handler!(hello));
///
/// // Bulk registration:
/// mola::routes!(router, hello, another_handler);
/// ```
#[macro_export]
macro_rules! routes {
    ($router:expr, $($handler:ident),+ $(,)?) => {
        $(
            $crate::paste! {
                { [<__mola_register_ $handler>] }(&mut $router);
            }
        )+
    };
}
