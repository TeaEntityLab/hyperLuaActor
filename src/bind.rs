use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

use futures::{Async, Future, Poll};
use hyper::header::{HeaderMap, HeaderName, HeaderValue};
use hyper::{Body, Error, Request, Response, Server, Uri};
use url::{ParseError, Url};

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
    hyper_request.insert("url".to_string(), LuaMessage::from(convert_url(request)));

    LuaMessage::from(hyper_request)
}

#[inline]
pub fn convert_headers(request: &Request<Body>) -> Vec<LuaMessage> {
    let mut data = Vec::<LuaMessage>::default();
    for item in request.headers().into_iter() {
        let (k, v) = item;

        let str_result = v.to_str();

        match str_result {
            Ok(_str) => {
                data.push(LuaMessage::from(vec![
                    LuaMessage::from(k.as_str()),
                    LuaMessage::from(_str),
                ]));
            }
            Err(_err) => {
                let bytes = v
                    .as_bytes()
                    .into_iter()
                    .map(|i| LuaMessage::from(*i))
                    .collect::<Vec<_>>();
                data.push(LuaMessage::from(vec![
                    LuaMessage::from(k.as_str()),
                    LuaMessage::from(bytes),
                ]));
            }
        }
    }

    data
}

#[inline]
pub fn convert_url(request: &Request<Body>) -> HashMap<&'static str, LuaMessage> {
    let mut data = HashMap::<&'static str, LuaMessage>::default();
    match Url::parse("") {
        Ok(parsed_url) => {
            let query_params: Vec<_> = parsed_url.query_pairs().into_owned().collect();
            data.insert(
                "query_params",
                LuaMessage::from(
                    query_params
                        .into_iter()
                        .map(|item| {
                            LuaMessage::from(vec![
                                LuaMessage::from(item.0),
                                LuaMessage::from(item.1),
                            ])
                        }).collect::<Vec<_>>(),
                ),
            );
        }
        Err(_err) => {}
    }

    data
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
