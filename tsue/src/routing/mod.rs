#![warn(missing_docs)]
//! Request routing.
//!
//! Routing is an operation that decide which [handler][self#handler] should handle current
//! request.
//!
//! ```
//! use tsue::routing::{Router, get};
//!
//! async fn index() -> &'static str {
//!     "Tsue Dev"
//! }
//!
//! let routes = Router::new()
//!     .route("/", get(index));
//! ```
//!
//! This will handle a `GET /` request with `200 OK` response and text body of `Tsue Dev`.
//!
//! # Router
//!
//! [`Router`] is the core type to build routes.
//!
//! To assign [handler][self#handler] to a route, use [`Router::route`] with corresponding method
//! router like [`get`], [`post`] etc.
//!
//! ```
//! use tsue::routing::{Router, get};
//! # async fn index() { }
//! # async fn add() { }
//!
//! let routes = Router::new()
//!     .route("/", get(index).post(add));
//! ```
//!
//! This will handle `GET /` request with `index` handler and `POST /` request with `add` handler.
//!
//! Note that handler must meet specific requirements, see [`Handler`][self#handler].
//!
//! ## Parameter
//!
//! In some cases, route can contains information, this is called route parameter.
//!
//! Part of the path can be prefixed with `:` to denote its a parameter, then use [`Params`] to
//! extract the value.
//!
//! ```
//! use tsue::{routing::{Router, get}, helper::Params};
//!
//! async fn index(Params(id): Params<i32>) {
//!     println!("Users with id: {id}");
//! }
//!
//! let routes = Router::new()
//!     .route("/users/:id", get(index));
//! ```
//!
//! For `GET /users/123` request, the `id` parameter will contains `123`.
//!
//! For more information on extracting parameter, see [`Params`].
//!
//! Note that parameter only available in [`Router::route`].
//!
//! ## Nesting
//!
//! In most cases, similar routes will be grouped with the same prefix, this is often called
//! route nesting.
//!
//! ```
//! # async fn add() { }
//! # async fn edit() { }
//! use tsue::routing::{Router, post};
//!
//! let routes = Router::new()
//!     .nest(
//!         "/post",
//!         Router::new()
//!             .route("/add", post(add))
//!             .route("/edit", post(edit))
//!     );
//!
//! // or
//!
//! let routes = Router::nested("/post")
//!     .route("/add", post(add))
//!     .route("/edit", post(edit));
//! ```
//!
//! This will handle `POST /post/add` request with `add` handler and `POST /post/edit` request with
//! `edit` handler.
//!
//! ## Composition
//!
//! Router is composable, meaning it can be declared separately and then merged into final routes.
//!
//! ```
//! # async fn users() { }
//! # async fn posts() { }
//! use tsue::routing::{Router, RouterService, get};
//!
//! fn user_routes() -> Router<impl RouterService> {
//!     Router::nested("/users")
//!         .route("/", get(users))
//! }
//!
//! fn post_routes() -> Router<impl RouterService> {
//!     Router::nested("/posts")
//!         .route("/", get(posts))
//! }
//!
//! let routes = Router::new()
//!     .merge(user_routes())
//!     .merge(post_routes());
//! ```
//!
//! Note that the router inner must implement [`RouterService`]. If you only use provided
//! [`Router`] methods, no need to worry about this. For more details, see [`RouterService`].
//!
//! # Handler
//!
//! Handler is an async function with arguments that implement [`FromRequest`] and returns type
//! that implement [`IntoResponse`].
//!
//! More specifically, the last argument must implement [`FromRequest`], while other arguments must
//! implement [`FromRequestParts`].
//!
//! ```
//! use http::Method;
//! use tsue::routing::get;
//!
//! // `String` implement `FromRequest`
//! async fn handler1(body: String) { }
//!
//! // `Method` implement `FromRequestParts`
//! async fn handler2(method: Method, body: String) { }
//!
//! // Anything that implement `FromRequestParts` also implement `FromRequest`
//! async fn handler3(method: Method) { }
//!
//! # let assert = get(handler1).post(handler2).put(handler3);
//! ```
//!
//! As for the return type, user can compose a tuple of [`IntoResponseParts`] to build
//! [`IntoResponse`] implementation.
//!
//! ```
//! use http::StatusCode;
//! use tsue::{response::IntoResponse, routing::get};
//!
//! // `String` implement `IntoResponse`
//! async fn handler1() -> String {
//!     String::new()
//! }
//!
//! // `StatusCode` implement `IntoResponseParts` which have blanket implementation of
//! // `IntoResponse`
//! async fn handler2() -> StatusCode {
//!     StatusCode::OK
//! }
//!
//! // Compose responses
//! async fn handler3() -> (StatusCode, String) {
//!     (StatusCode::OK, String::new())
//! }
//!
//! # let assert = get(handler1).post(handler2).put(handler3);
//! ```
//!
//! Note on `impl IntoResponse`, it can only represent one type, so the following example did not
//! compile.
//!
//! ```compile_fail
//! use http::{Method, StatusCode};
//! use tsue::response::IntoResponse;
//!
//! async fn handler1(method: Method) -> impl IntoResponse {
//!     if method != Method::GET {
//!         return StatusCode::NOT_FOUND
//!     }
//!
//!     String::new()
//!     // ^^^^^^^^^^ expected `StatusCode`, found `String`
//! }
//! ```
//!
//! For conditional response, consider [`Result`] or [`Either`].
//!
//! ```
//! use http::{Method, StatusCode};
//! use tsue::{response::IntoResponse, helper::Either, routing::get};
//!
//! async fn handler1(method: Method) -> impl IntoResponse {
//!     if method != Method::GET {
//!         return Err(StatusCode::NOT_FOUND)
//!     }
//!
//!     Ok(String::new())
//! }
//!
//! async fn handler2(body: String) -> impl IntoResponse {
//!     if body == "foo" {
//!         Either::Left(String::from("bar"))
//!     } else {
//!         Either::Right(StatusCode::NOT_FOUND)
//!     }
//! }
//!
//! # let assert = get(handler1).post(handler2);
//! ```
//!
//! If any of the requirements did not meet, it will fail to compile.
//!
//! ```compile_fail
//! use http::Method;
//! use tsue::routing::get;
//!
//! // handler must be async
//! fn handler() { }
//!
//! // `i32` does not implement `FromRequestParts` nor `FromRequest`
//! async fn handler1(body: i32) { }
//!
//! // `FromRequest` implementation must be the last argument
//! async fn handler2(body: String, method: Method) { }
//!
//! // only one `FromRequest` implementation are allowed
//! async fn handler3(body1: String, body2: String) { }
//!
//! // `i32` does not implement `IntoResponse` nor `IntoResponseParts`
//! async fn handler4() -> i32 { 0 }
//!
//! # let assert = get(handler).get(handler1).post(handler2).put(handler3).get(handler4);
//! ```
//!
//! # Middleware
//!
//! To extract information from a request, it is recommended to use [`FromRequest`] implementation
//! and use it in appropriate handlers. But in some cases, users need to run a logic for every
//! request.
//!
//! The standard way is to create a type which implement [`Service<Request>`] and [`Layer`], and use
//! [`Router::layer`] to assign it. The easy way is to use [`Router::middleware`] and provide async
//! function containing the logic.
//!
//! ```
//! # use http::StatusCode;
//! # use tsue::{
//! #     request::Request,
//! #     response::{Response, IntoResponse},
//! #     routing::{Router, get},
//! # };
//! use tsue::routing::Next;
//!
//! async fn check_maintenance(req: Request, next: Next) -> Result<Response, StatusCode> {
//!     if is_maintenance(&req) {
//!         return Err(StatusCode::SERVICE_UNAVAILABLE);
//!     }
//!
//!     Ok(next.next(req).await)
//! }
//!
//! let routes = Router::new()
//!     .route("/", get(async || "Ok"))
//!     .middleware(check_maintenance);
//! #
//! # fn is_maintenance(req: &Request) -> bool { false }
//! ```
//!
//! Note on the order of the services, it runs from bottom to top of assgined services.
//!
//! ```
//! # use http::StatusCode;
//! # use tsue::{
//! #     request::Request,
//! #     response::{Response, IntoResponse},
//! #     routing::{Router, get},
//! # };
//! use tsue::routing::Next;
//!
//! async fn check_maintenance(req: Request, next: Next) -> Result<Response, StatusCode> {
//!     if is_maintenance(&req) {
//!         return Err(StatusCode::SERVICE_UNAVAILABLE);
//!     }
//!
//!     Ok(next.next(req).await)
//! }
//!
//! let routes = Router::new()
//!     .route("/", get(async || "Ok"))
//!     .middleware(check_maintenance)
//!     .route("/status", get(async |req| if is_maintenance(&req) {
//!         "Maintenance"
//!     } else {
//!         "Ok"
//!     }));
//! #
//! # fn is_maintenance(req: &Request) -> bool { false }
//! ```
//!
//! The `/status` route will not run through the `check_maintenance` middleware.
//!
//! [`Params`]: crate::helper::Params
//! [`FromRequest`]: crate::request::FromRequest
//! [`FromRequestParts`]: crate::request::FromRequestParts
//! [`IntoResponse`]: crate::response::IntoResponse
//! [`IntoResponseParts`]: crate::response::IntoResponseParts
//! [`Either`]: crate::helper::Either
//! [`Service<Request>`]: crate::service::Service
//! [`Layer`]: crate::service::Layer

// shared state
mod matcher;
#[cfg(feature = "serde")]
pub(crate) mod extract;
mod zip;

// core routings
mod router;
mod fallback;
mod middleware;
mod branch;
mod nest;

// async fn
mod handler;

// utilities
mod state;

#[cfg(feature = "tokio")]
mod adapter;

// ===== reexports =====

pub(crate) use zip::Zip;

pub use router::Router;
pub use middleware::{Next, NextFuture};
pub use branch::{get, post, put, patch, delete};
pub use state::State;

#[cfg(feature = "tokio")]
pub(crate) use adapter::Hyper;

#[doc(inline)]
pub use crate::service::RouterService;
