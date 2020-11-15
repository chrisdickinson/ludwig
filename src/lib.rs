use std::collections::HashMap;
use std::pin::Pin;
use std::borrow::Cow;
use futures::Future;
use futures::prelude::*;
use std::marker::PhantomData;
use route_recognizer::{Router, Params};
use std::sync::Arc;
mod responses;
mod context;

pub use crate::responses::*;
pub use crate::context::*;

pub type HandlerFuture<'a> = Pin<Box<dyn Future<Output = Response<'a>> + Send + Sync + 'a>>;
pub type HandlerFunction<'a, AppState, RequestState> = Box<dyn Fn(Context<AppState, RequestState>) -> HandlerFuture<'a> + Send + Sync>;

pub struct Handler<'a, AppState: Send + Sync, RequestState: Send + Sync> {
    name: String,
    method: String,
    route: String,
    pub action: HandlerFunction<'a, AppState, RequestState>
}

impl<'a> From<()> for Response<'a> {
    fn from(_: ()) -> Response<'a> {
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

impl<'a, T, K, V> From<(u16, HashMap<K, V>, T)> for Response<'a>
    where
        T: Into<Response<'a>>,
        K: Into<Cow<'a, str>>,
        V: Into<Cow<'a, str>> {
    fn from(value: (u16, HashMap<K, V>, T)) -> Response<'a> {
        let mut response = value.2.into();
        response.status = value.0;
        for (k, v) in value.1 {
            response.headers.insert(k.into(), v.into());
        }
        response
    }
}

impl<'a, AS, RS, Name, Method, Route, F, Fut, E> From<(Name, Method, Route, F)> for Handler<'a, AS, RS>
where 
    AS: Send + Sync,
    RS: Send + Sync,
    Name: AsRef<str>,
    Method: AsRef<str>,
    Route: AsRef<str>,
    F: (Fn(Context<AS, RS>) -> Fut) + Send + Sync + 'static,
    Fut: Future<Output = E> + Send + Sync + 'static,
    E: Send + Sync + Into<Response<'a>> {
    fn from(value: (Name, Method, Route, F)) -> Handler<'a, AS, RS> {
        let (name, method, route, handler) = value;
        let name = name.as_ref().to_string();
        let method = method.as_ref().to_string();
        let route = route.as_ref().to_string();
        Handler {
            name,
            method,
            route,
            action: Box::new(move |context| {
                Box::pin(handler(context).map(|xs| xs.into()))
            })
        }
    }
}

pub struct Application<'a, ApplicationState: Send + Sync, RequestState: Default + Send + Sync> {
    app_state: Arc<ApplicationState>,
    router: Router<usize>,
    handlers: Vec<Handler<'a, ApplicationState, RequestState>>
}

impl<'a, ApplicationState, RequestState> Application<'a, ApplicationState, RequestState> 
    where ApplicationState: Send + Sync,
          RequestState: Default + Send + Sync {
    pub fn new(app_state: ApplicationState) -> Self {
        Application {
            app_state: Arc::new(app_state),
            router: Router::new(),
            handlers: Vec::new()
        }
    }

    pub fn route<T: Into<Handler<'a, ApplicationState, RequestState>>>(mut self, handler: T) -> Self {
        let handler = handler.into();
        let route = handler.route.as_ref();
        self.router.add(route, self.handlers.len());
        self.handlers.push(handler);
        self
    }

    pub async fn execute(&self, req: http_types::Request) -> Option<Response<'a>> {
        if let Ok(matchinfo) = self.router.recognize(req.url().path()) {
            let context = Context::new(self.app_state.clone(), Default::default(), HashMap::new());
            Some((self.handlers[**matchinfo.handler()].action)(context).await)
        } else {
            None
        }
    }
}
