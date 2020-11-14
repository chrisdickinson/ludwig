#![feature(async_closure)]

use std::collections::HashMap;
use std::sync::Arc;
use std::pin::Pin;
use std::borrow::Cow;
use futures::Future;
use futures::prelude::*;


// - middleware produces a handler
// - handlers accept context and produce a response
// - responses are anything that we know how to turn into a http response
// - it is the frameworks job to liberally accept whatever handlers return and 
//   represent it as a "response" enum
//
// acceptable responses:
// - [-] streams of data (NB: this is tricky because streams impl From for (), strings that
//   conflict with our impls)
// - [x] json
// - [x] strings
// - [x] static strs
// - [x] static slices
// - [x] vec<u8>
// - [x] impl std::error::Error
// - [ ] HashMap< String, any of the above >
// - [ ] Vec< of any of the above >
// - [x] unit
// - [x] tuple (u16, <any of the above>)
// - [x] tuple (u16, HashMap<String, String>, <any of the above>)
// - [ ] Body of any of the above
// - [ ] Result< of any of the above >

pub enum Body {
    Empty,
    ByteSlice(&'static [u8]),
    ByteVec(Vec<u8>),
    Str(&'static str),
    String(String),
    JSON(serde_json::Value),
    Error(Box<dyn std::error::Error>),
    Map(HashMap<String, Body>),
    List(Vec<Body>),
}

impl Into<http_types::Body> for Body {
    fn into(self) -> http_types::Body {
        match self {
            Body::Empty => http_types::Body::empty(),
            Body::Str(xs) => xs.into(),
            Body::String(xs) => xs.into(),
            Body::JSON(xs) => xs.into(),
            _ => unimplemented!("oh no")
        }
    }
}

impl Default for Body {
    fn default() -> Self {
        Body::Empty
    }
}

// XXX: maybe structure this as a tuple?
#[derive(Default)]
pub struct Response<'a> {
    pub body: Body,
    pub headers: HashMap<Cow<'a, str>, Cow<'a, str>>,
    pub status: u16
}

pub struct Context<AppState, RequestState> {
    application: Arc<AppState>,
    context: RequestState,
    headers: HashMap<String, String>,
}

impl<AppState, RequestState> Context<AppState, RequestState> {
    pub fn new(application: Arc<AppState>, context: RequestState, headers: HashMap<String, String>) -> Self {
        Context {
            application,
            context,
            headers
        }
    }
}

pub struct Handler<'a, AppState: Send + Sync, RequestState: Send + Sync> {
    name: String,
    method: String,
    route: String,
    pub action: Box<dyn Fn(Context<AppState, RequestState>) -> Pin<Box<dyn Future<Output = Response<'a>> + Send + Sync + 'a>> + Send + Sync>
}

impl<'a> From<()> for Response<'a> {
    fn from(value: ()) -> Response<'a> {
        Response {
            status: 204,
            body: Body::Empty,
            ..Default::default()
        }
    }
}

impl<'a> From<&'static str> for Response<'a> {
    fn from(value: &'static str) -> Response<'a> {
        Response {
            status: 200,
            body: Body::Str(value),
            headers: maplit::hashmap!(
                "content-type".into() => "text/plain; charset=utf-8".into()
            )
        }
    }
}

impl<'a> From<String> for Response<'a> {
    fn from(value: String) -> Response<'a> {
        Response {
            status: 200,
            body: Body::String(value),
            headers: maplit::hashmap!(
                "content-type".into() => "text/plain; charset=utf-8".into()
            )
        }
    }
}

impl<'a> From<serde_json::Value> for Response<'a> {
    fn from(value: serde_json::Value) -> Response<'a> {
        Response {
            status: 200,
            body: Body::JSON(value),
            headers: maplit::hashmap!(
                "content-type".into() => "application/json; charset=utf-8".into()
            )
        }
    }
}

impl<'a> From<&'static [u8]> for Response<'a> {
    fn from(value: &'static [u8]) -> Response<'a> {
        Response {
            status: 200,
            body: Body::ByteSlice(value),
            headers: maplit::hashmap!(
                "content-type".into() => "application/octet-stream".into()
            )
        }
    }
}

impl<'a> From<Vec<u8>> for Response<'a> {
    fn from(value: Vec<u8>) -> Response<'a> {
        Response {
            status: 200,
            body: Body::ByteVec(value),
            headers: maplit::hashmap!(
                "content-type".into() => "application/octet-stream".into()
            )
        }
    }
}

impl<'a, T> From<Option<T>> for Response<'a>
    where T: Into<Response<'a>> {
    fn from(value: Option<T>) -> Response<'a> {
        value.map(|xs| xs.into()).unwrap_or_else(|| ().into())
    }
}

impl<'a, T, E> From<Result<T, E>> for Response<'a>
    where T: Into<Response<'a>>,
          E: Into<anyhow::Error> {
    fn from(value: Result<T, E>) -> Response<'a> {
        match value {
            Ok(xs) => xs.into(),
            Err(e) => Response {
                status: 500,
                body: Body::String(e.into().to_string()),
                headers: maplit::hashmap!(
                    "content-type".into() => "text/plain; charset=utf-8".into()
                )
            }
        }
    }
}

impl<'a, T> From<(u16, T)> for Response<'a>
    where T: Into<Response<'a>> {
    fn from(value: (u16, T)) -> Response<'a> {
        let mut response = value.1.into();
        response.status = value.0;
        response
    }
}

impl<'a, T> From<(u16, HashMap<Cow<'a, str>, Cow<'a, str>>, T)> for Response<'a>
    where T: Into<Response<'a>> {
    fn from(value: (u16, HashMap<Cow<'a, str>, Cow<'a, str>>, T)) -> Response<'a> {
        let mut response = value.2.into();
        response.status = value.0;
        response.headers.extend(value.1);
        response
    }
}

impl<'a, AppState: Send + Sync, RequestState: Send + Sync> Handler<'a, AppState, RequestState> {
    pub fn new<Fut, F, E>(name: String, method: String, route: String, handler: F) -> anyhow::Result<Self>
        where F: (Fn(Context<AppState, RequestState>) -> Fut) + Send + Sync + 'static,
              Fut: Future<Output = E> + Send + Sync + 'static,
              E: Send + Sync + Into<Response<'a>> {

        Ok(Handler {
            name,
            method,
            route,
            action: Box::new(move |context| {
                Box::pin(handler(context).map(|xs| xs.into()))
            })
        })
    }
}
