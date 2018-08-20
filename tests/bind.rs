#[macro_use]
extern crate futures;
extern crate fp_rust;
extern crate hyper;
extern crate tokio;

extern crate hyper_lua_actor;

use hyper::rt::Future;
use hyper::service::service_fn_ok;
use hyper::{Body, Response, Server};
use tokio::runtime::current_thread::Runtime;

use hyper_lua_actor::bind::HyperLatch;

#[test]
fn test_get_header() {
    static TEXT: &str = "Hello, World!";

    let addr = ([127, 0, 0, 1], 3000).into();

    let new_svc = || service_fn_ok(|_req| Response::new(Body::from(TEXT)));

    let server = Server::bind(&addr).serve(new_svc);
    let fut = server
        .select(HyperLatch::default())
        .then(|_| Ok::<(), ()>(()));

    let mut rt = Runtime::new().expect("rt new");
    rt.block_on(fut).unwrap();

    // hyper::rt::run(server.map_err(|e| eprintln!("server error: {}", e)));

    println!("OK");
}
