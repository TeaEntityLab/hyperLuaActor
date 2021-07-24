use std::collections::HashMap;
use std::pin::Pin;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::task::{Context, Poll};

use futures::Future;
// use futures::TryStreamExt;
use hyper::body;
use hyper::{Body, Request};
use url::Url;

use mlua;
use mlua_actor::actor::Actor;
use mlua_actor::message::LuaMessage;

#[inline]
pub fn call_hyper_request(
    actor: &mut Actor,
    request: &mut Request<Body>,
    // skip_parse_boody_as_string: bool,
) -> Result<LuaMessage, mlua::Error> {
    // setup_hyper_get_request_body(actor, request, skip_parse_boody_as_string);
    actor.call(
        "hyper_request",
        get_hyper_request_lua_message(request).into(),
    )
}

#[inline]
pub fn call_hyper_request_nowait(
    actor: &Actor,
    request: &mut Request<Body>,
    // skip_parse_boody_as_string: bool,
) {
    // setup_hyper_get_request_body(actor, request, skip_parse_boody_as_string);
    let _ = actor.call_nowait(
        "hyper_request",
        get_hyper_request_lua_message(request).into(),
    );
}

#[inline]
pub async fn setup_hyper_get_request_body(
    actor: &Actor,
    request: &mut Request<Body>,
    use_raw_data: bool,
) {
    let lua_arc = actor.lua();
    let lua_guard = lua_arc.lock();
    let lua = lua_guard.as_ref().unwrap();

    let body_raw = request.body_mut();
    let bytes = body::to_bytes(body_raw).await;
    // Raw content
    /*
    let bytes = body_raw
        .try_fold(Vec::new(), |mut data, chunk| async move {
            data.extend_from_slice(&chunk);
            Ok(data)
        })
        .await;
    */
    let bytes_vec = bytes.ok().unwrap().to_vec();

    if use_raw_data {
        let _ = actor.def_fn_with_name_sync(
            lua,
            // NOTE: mlua::Value Could be mlua::Value::Nil -> for empty parameter
            move |_, _input: mlua::Value| Ok(bytes_vec.clone()),
            "hyper_get_request_body",
        );
    } else {
        let body_str: String;
        // unsafe { body_str = String::from_utf8_unchecked(bytes_vec.clone()) };
        body_str = String::from_utf8(bytes_vec).ok().unwrap();

        let _ = actor.def_fn_with_name_sync(
            lua,
            // NOTE: mlua::Value Could be mlua::Value::Nil -> for empty parameter
            move |_, _input: mlua::Value| Ok(body_str.clone()),
            "hyper_get_request_body",
        );
    }
}

#[inline]
pub fn get_hyper_request_lua_message(request: &Request<Body>) -> LuaMessage {
    let mut hyper_request = HashMap::<String, LuaMessage>::default();
    hyper_request.insert(
        "headers".to_string(),
        LuaMessage::from(convert_headers(request)),
    );
    hyper_request.insert(
        "url_meta".to_string(),
        LuaMessage::from(convert_url(request)),
    );
    hyper_request.insert(
        "method".to_string(),
        LuaMessage::from(request.method().to_string()),
    );
    hyper_request.insert(
        "version".to_string(),
        LuaMessage::from(format!("{:?}", request.version())),
    );
    hyper_request.insert(
        "uri".to_string(),
        LuaMessage::from(request.uri().to_string()),
    );
    hyper_request.insert(
        "extensions".to_string(),
        LuaMessage::from(format!("{:?}", request.extensions())),
    );

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
pub fn convert_url(request: &Request<Body>) -> HashMap<String, LuaMessage> {
    let mut data = HashMap::<String, LuaMessage>::default();

    let url = request.uri().to_string();
    data.insert("url_raw".to_string(), url.clone().into());

    match Url::parse(url.as_str()) {
        Ok(parsed_url) => {
            let query_params: Vec<_> = parsed_url.query_pairs().into_owned().collect();
            data.insert(
                "query_params".to_string(),
                LuaMessage::from(
                    query_params
                        .into_iter()
                        .map(|item| {
                            LuaMessage::from(vec![
                                LuaMessage::from(item.0),
                                LuaMessage::from(item.1),
                            ])
                        })
                        .collect::<Vec<_>>(),
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
    type Output = ();
    // type Error = Error;

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.is_alive.lock().unwrap().load(Ordering::SeqCst) {
            Poll::Pending
        } else {
            Poll::Ready(())
        }
    }
}
