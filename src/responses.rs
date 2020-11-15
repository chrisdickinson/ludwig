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

use std::collections::HashMap;
use std::sync::Arc;
use std::pin::Pin;
use std::borrow::Cow;
use futures::Future;
use futures::prelude::*;

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
