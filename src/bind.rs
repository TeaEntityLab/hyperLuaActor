use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

use futures::{Async, Future, Poll};
use hyper::header::{HeaderMap, HeaderName, HeaderValue};
use hyper::{Body, Error, Request, Response, Server};

use lua_actor::actor::Actor;
use lua_actor::message::LuaMessage;
use rlua;

pub fn call_hyper_request(
    actor: &mut Actor,
    request: &Request<Body>,
) -> Result<LuaMessage, rlua::Error> {
    actor.call("hyper_request", get_hyper_request_lua_message(request))
}

pub fn call_hyper_request_nowait(actor: &mut Actor, request: &Request<Body>) {
    let _ = actor.call_nowait("hyper_request", get_hyper_request_lua_message(request));
}

#[inline]
pub fn get_hyper_request_lua_message(request: &Request<Body>) -> LuaMessage {
    let mut hyper_request = HashMap::<String, LuaMessage>::default();
    hyper_request.insert(
        "headers".to_string(),
        LuaMessage::from(convert_headers(request)),
    );

    LuaMessage::from(hyper_request)
}

#[inline]
pub fn convert_headers(request: &Request<Body>) -> HashMap<String, LuaMessage> {
    let mut headers = HashMap::<String, LuaMessage>::default();
    for item in request.headers().into_iter() {
        let (k, v) = item;

        let str_result = v.to_str();

        match str_result {
            Ok(_str) => {
                headers.insert(k.as_str().to_string(), LuaMessage::from(_str));
            }
            Err(_err) => {
                let bytes = v
                    .as_bytes()
                    .into_iter()
                    .map(|i| LuaMessage::from(*i))
                    .collect::<Vec<_>>();
                headers.insert(k.as_str().to_string(), LuaMessage::from(bytes));
            }
        }
    }

    headers
}

#[derive(Debug, Clone)]
pub struct HyperLatch {
    is_alive: Arc<Mutex<AtomicBool>>,
}
impl HyperLatch {
    pub fn mark_done(&self) {
        self.is_alive.lock().unwrap().store(false, Ordering::SeqCst);
    }
}
impl Default for HyperLatch {
    fn default() -> Self {
        HyperLatch {
            is_alive: Arc::new(Mutex::new(AtomicBool::new(true))),
        }
    }
}
impl Future for HyperLatch {
    type Item = ();
    type Error = Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        if self.is_alive.lock().unwrap().load(Ordering::SeqCst) {
            Ok(Async::NotReady)
        } else {
            Ok(Async::Ready(()))
        }
    }
}
