extern crate fp_rust;
extern crate futures;
extern crate hyper;
extern crate tokio;

extern crate hyper_lua_actor;
extern crate lua_actor;

use std::io::{Read, Write};
use std::net::{IpAddr, Ipv4Addr, Shutdown, SocketAddr, TcpStream};
use std::thread;
use std::time::Duration;

use hyper::rt::Future;
use hyper::service::service_fn_ok;
use hyper::{Body, Response, Server};
use tokio::runtime::current_thread::Runtime;

use fp_rust::sync::CountDownLatch;
use hyper_lua_actor::bind::*;
use lua_actor::actor::Actor;

fn connect(addr: &SocketAddr) -> TcpStream {
    let req = TcpStream::connect(addr).unwrap();
    req.set_read_timeout(Some(Duration::from_secs(1))).unwrap();
    req.set_write_timeout(Some(Duration::from_secs(1))).unwrap();
    req
}

#[test]
fn test_get_header() {
    // let actor = Actor::new_with_handler(None);
    let actor = Actor::new();
    let started_latch = CountDownLatch::new(1);
    let hyper_latch = HyperLatch::default();

    let _ = actor.exec_nowait(
        r#"
            i = 0
        "#,
        None,
    );
    let _ = actor.exec_nowait(
        r#"
            function tablelength(T)
                local count = 0
                for _ in pairs(T) do count = count + 1 end
                return count
            end
            function hyper_request (req)
                i = 0
                -- i = table.getn(req)
                i = tablelength(req)
            end
        "#,
        None,
    );

    let started_latch_for_thread = started_latch.clone();
    let hyper_latch_for_thread = hyper_latch.clone();
    let actor_for_thread = actor.clone();
    thread::spawn(move || {
        static TEXT: &str = "Hello, World!";

        let addr = ([127, 0, 0, 1], 3000).into();

        let started_latch = started_latch_for_thread.clone();
        let actor = actor_for_thread.clone();
        let new_svc = move || {
            let started_latch = started_latch.clone();
            let actor = actor.clone();
            service_fn_ok(move |_req| {
                let mut actor = actor.clone();
                call_hyper_request_nowait(&mut actor, &_req);

                started_latch.countdown();

                Response::new(Body::from(TEXT))
            })
        };

        let started_latch = started_latch_for_thread.clone();
        let server = Server::bind(&addr).serve(new_svc);
        let fut = server.select(hyper_latch_for_thread).then(move |_| {
            // started_latch.countdown();

            Ok::<(), ()>(())
        });

        let mut rt = Runtime::new().expect("rt new");
        rt.block_on(fut).unwrap();
    });

    // hyper::rt::run(server.map_err(|e| eprintln!("server error: {}", e)));

    let addr = ([127, 0, 0, 1], 3000).into();
    let mut req = connect(&addr);
    req.write_all(
        b"\
        GET / HTTP/1.1\r\n\
        Host: example.domain\r\n\
        Content-Length: 19\r\n\
        \r\n\
        I'm a good request.\r\n\
    ",
    ).unwrap();
    req.read(&mut [0; 256]).unwrap();

    started_latch.wait();

    assert_ne!(Some(0), Option::from(actor.get_global("i").ok().unwrap()));
    hyper_latch.mark_done();

    println!("OK");
}
