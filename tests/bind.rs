extern crate fp_rust;
extern crate futures;
extern crate hyper;

extern crate hyper_lua_actor;
extern crate lua_actor;

use std::net::SocketAddr;

use futures::executor::block_on;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Client, Method, Request, Response, Server};

use fp_rust::sync::CountDownLatch;
use hyper_lua_actor::bind::*;
// use hyper_lua_actor::blocking_future;
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
            request_params_len = 0
            headers_len = 0
            uri = ''
            request_body = ''
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
                -- request_params_len = table.getn(req)
                request_params_len = tablelength(req)
                headers_len = tablelength(req['headers'])
                uri = req['uri']
                request_body = hyper_get_request_body()
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
            Ok::<_, hyper::Error>(service_fn(move |mut req: Request<Body>| {
                let actor_for_thread_4 = actor_for_thread_3.clone();
                let started_latch_for_thread_3 = started_latch_for_thread_2.clone();

                async move {
                    println!("StartedB");
                    let mut actor = actor_for_thread_4.clone();
                    block_on(setup_hyper_get_request_body(&mut actor, &mut req, false));
                    call_hyper_request_nowait(&mut actor, &mut req);

                    println!("Started");

                    started_latch_for_thread_3.countdown();

                    let response = Response::new(Body::from(TEXT));
                    Ok::<Response<Body>, hyper::Error>(response)
                }
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
    let request = Request::builder()
        .method(Method::POST)
        .uri("http://".to_string() + &addr.to_string())
        .header("content-type", "application/json")
        .body(Body::from(r#"{"library":"hyper"}"#))
        .ok()
        .unwrap();

    println!("{:?}", request);
    let resp = client.request(request).await;
    let resp_ref = resp.as_ref();
    let err = resp_ref.err();
    println!("{:?}", err);
    assert_eq!(false, resp_ref.is_err());

    started_latch.wait();
    println!("REQ",);

    let request_params_len =
        Option::<i64>::from(actor.get_global("request_params_len").ok().unwrap());
    let headers_len = Option::<i64>::from(actor.get_global("headers_len").ok().unwrap());
    let uri = actor.get_global("uri").ok().unwrap();
    let request_body = actor.get_global("request_body").ok().unwrap();
    println!(
        "request_params_len={:?}, headers_len={:?}, uri={:?}, request_body={:?}",
        request_params_len, headers_len, uri, request_body,
    );
    assert_ne!(Some(0), request_params_len);
    assert_ne!(Some(0), headers_len);
    assert_ne!(Some("".to_string()), Option::<String>::from(uri));
    assert_ne!(Some("".to_string()), Option::<String>::from(request_body));

    hyper_latch.mark_done();
    // hyper_latch.countdown();

    println!("OK");
}
