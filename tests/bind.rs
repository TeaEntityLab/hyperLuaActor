extern crate fp_rust;
extern crate futures;
extern crate hyper;

extern crate hyper_lua_actor;
extern crate lua_actor;

use std::net::SocketAddr;

use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Client, Request, Response, Server};

use fp_rust::sync::CountDownLatch;
use hyper_lua_actor::bind::*;
use lua_actor::actor::Actor;

/*
fn connect(addr: &SocketAddr) -> std::io::Result<TcpStream> {
    let req = TcpStream::connect(addr)?;
    req.set_read_timeout(Some(Duration::from_secs(1))).unwrap();
    req.set_write_timeout(Some(Duration::from_secs(1))).unwrap();
    Ok(req)
}
*/

#[tokio::test]
async fn test_get_header() {
    use tokio::time::{sleep, Duration};

    // let actor = Actor::new_with_handler(None);
    let actor = Actor::new();
    let started_latch = CountDownLatch::new(1);
    // let hyper_latch = CountDownLatch::new(1);
    let hyper_latch = HyperLatch::default();

    let _ = actor.exec_nowait(
        r#"
            i = 0
        "#,
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
    );

    let addr: SocketAddr = ([127, 0, 0, 1], 3000).into();

    let started_latch_for_thread = started_latch.clone();
    let hyper_latch_for_thread = hyper_latch.clone();
    let actor_for_thread = actor.clone();

    static TEXT: &str = "Hello, World!";

    let actor_for_thread_2 = actor_for_thread.clone();
    let server = Server::bind(&addr).serve(make_service_fn(move |_| {
        let actor_for_thread_3 = actor_for_thread_2.clone();
        let started_latch_for_thread_2 = started_latch_for_thread.clone();
        async {
            Ok::<_, hyper::Error>(service_fn(move |req: Request<Body>| {
                let actor_for_thread_4 = actor_for_thread_3.clone();
                let started_latch_for_thread_3 = started_latch_for_thread_2.clone();

                println!("StartedB");
                let mut actor = actor_for_thread_4.clone();
                call_hyper_request_nowait(&mut actor, &req);

                println!("Started");

                started_latch_for_thread_3.countdown();

                async { Ok::<Response<Body>, hyper::Error>(Response::new(Body::from(TEXT))) }
            }))
        }
    }));

    println!("Started C");

    tokio::spawn(async {
        // hyper_latch_for_thread.countdown();
        let _ = server
            .with_graceful_shutdown(async move {
                hyper_latch_for_thread.await;
            })
            .await;
    });

    sleep(Duration::from_millis(200)).await;

    /*
    let mut req = connect(&addr).unwrap();
    req.write_all(
        b"\
        GET / HTTP/1.1\r\n\
        Host: example.domain\r\n\
        Content-Length: 19\r\n\
        \r\n\
        I'm a good request.\r\n\
    ",
    )
    .unwrap();
    req.read(&mut [0; 256]).unwrap();
    */

    let client = Client::new();
    let resp = client
        .get(("http://".to_string() + &addr.to_string()).parse().unwrap())
        .await;
    let resp_ref = resp.as_ref();
    let err = resp_ref.err();
    println!("{:?}", err);
    assert_eq!(false, resp_ref.is_err());

    started_latch.wait();
    println!("REQ",);

    let i_val = Option::from(actor.get_global("i").ok().unwrap());
    println!("i={:?}", i_val);
    assert_ne!(Some(0), i_val);

    hyper_latch.mark_done();
    // hyper_latch.countdown();

    println!("OK");
}
