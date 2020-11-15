use std::sync::Arc;
use std::collections::HashMap;
use route_recognizer::Params;

pub struct Context<AppState, RequestState> {
    app_state: Arc<AppState>,
    request_state: RequestState,
    params: Params,
    request: http_types::Request
}

impl<AppState, RequestState> Context<AppState, RequestState> {
    pub fn new(app_state: Arc<AppState>, request_state: RequestState, params: Params, request: http_types::Request) -> Self {
        Context {
            app_state,
            request_state,
            request,
            params,
        }
    }

    pub fn app_state(&self) -> &AppState {
        &*self.app_state
    }

    pub fn app_state_mut(&mut self) -> Option<&mut AppState> {
        Arc::get_mut(&mut self.app_state)
    }

    pub fn request_state(&self) -> &RequestState {
        &self.request_state
    }

    pub fn url(&self) -> &http_types::Url {
        self.request.url()
    }

    pub fn url_mut(&mut self) -> &mut http_types::Url {
        self.request.url_mut()
    }

    pub fn params(&self) -> &Params {
        &self.params
    }

    pub fn params_mut(&mut self) -> &mut Params {
        &mut self.params
    }

    pub fn request_state_mut(&mut self) -> &mut RequestState {
        &mut self.request_state
    }
}

pub type EmptyContext = Context<(), ()>;
pub type AppContext<T> = Context<T, ()>;
