#![feature(async_closure)]

use std::collections::HashMap;
use std::sync::Arc;

// - middleware produces a handler
// - handlers accept context and produce a response
// - responses are anything that we know how to turn into a http response
// - it is the frameworks job to liberally accept whatever handlers return and 
//   represent it as a "response" enum
//
// acceptable responses:
// - [x] streams of data
// - [x] json
// - [x] strings
// - [x] static strs
// - [x] static slices
// - [x] vec<u8>
// - [x] impl std::error::Error
// - [x] HashMap< String, any of the above >
// - [ ] Vec< of any of the above >
// - [ ] unit
// - [ ] tuple (u16, <any of the above>)
// - [ ] tuple (HashMap<String, String>, <any of the above>)
// - [ ] tuple (u16, HashMap<String, String>, <any of the above>)
// - [ ] Body of any of the above
// - [ ] Result< of any of the above >

enum Body {
    Empty,
    ByteStream(Box<dyn futures::stream::Stream<Item = Vec<u8>>>),
    ByteSlice(&'static [u8]),
    ByteVec(Vec<u8>),
    Str(&'static str),
    String(String),
    JSON(serde_json::Value),
    Error(Box<dyn std::error::Error>),
    Map(HashMap<String, Body>),
    List(Vec<Body>),
}

// XXX: maybe structure this as a tuple?
struct Response {
    body: Body,
    headers: HashMap<String, String>,
    status: u16
}

struct Context<AppState, RequestState> {
    application: Arc<AppState>,
    context: RequestState,
    headers: HashMap<String, String>,
}

use std::pin::Pin;
use futures::Future;

struct Handler<AppState, RequestState> {
    name: String,
    method: String,
    route: String,
    action: Box<dyn Fn(Context<AppState, RequestState>) -> anyhow::Result<Response>>
}

async fn cast<AppState, RequestState, F, E>(context: Context<AppState, RequestState>, func: impl Fn(Context<AppState, RequestState>) -> F) -> anyhow::Result<Response>
    where F: futures::Future<Output = anyhow::Result<E>>,
          E: Into<Response> {
    func(context).await.map(Into::into)
}

impl<AppState, RequestState> Handler<AppState, RequestState> {
    fn new<F, E>(name: String, method: String, route: String, handler: F) -> anyhow::Result<Self>
        where F: 'static + Fn(Context<AppState, RequestState>) -> anyhow::Result<E>,
              E: Into<Response> {

        let action = Box::new(move |context| {
            handler(context).map(Into::into)
        });

        Ok(Handler {
            name,
            method,
            route,
            action
        })
    }
}
