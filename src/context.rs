use std::sync::Arc;
use std::collections::HashMap;

pub struct Context<AppState, RequestState> {
    app_state: Arc<AppState>,
    request_state: RequestState,
    headers: HashMap<String, String>,
}

impl<AppState, RequestState> Context<AppState, RequestState> {
    pub fn new(app_state: Arc<AppState>, request_state: RequestState, headers: HashMap<String, String>) -> Self {
        Context {
            app_state,
            request_state,
            headers
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

    pub fn request_state_mut(&mut self) -> &mut RequestState {
        &mut self.request_state
    }

    pub fn headers(&self) -> &HashMap<String, String> {
        &self.headers
    }
}

pub type EmptyContext = Context<(), ()>;
pub type AppContext<T> = Context<T, ()>;
