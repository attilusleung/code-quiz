use executor::config::{CONFIG, verify_config};
use std::time::{Duration, Instant};
use std::str::from_utf8;
use tokio::sync::{Mutex, Semaphore};
use std::net::SocketAddr;
use warp::{Filter, http::{Response, StatusCode}};
use log::{info, trace};
use lazy_static::lazy_static;
use std::fs::read_to_string;
use handlebars::Handlebars;
use serde_json::json;

lazy_static! {
    static ref TIMEOUT: Duration = Duration::from_millis(CONFIG.timeout as u64);
    static ref STATIC_CONTENT: String = Handlebars::new()
        .render_template(
            &read_to_string(&CONFIG.template).expect("Failed to read template file"),
            &json!({"questions": &CONFIG.questions})
        ).expect("Failed to render template");
    static ref CACHE: Mutex<lru::LruCache<SocketAddr, Instant>> = Mutex::new(lru::LruCache::new(100));
    static ref BOUND: Semaphore = Semaphore::new(CONFIG.max_proc as usize);
}


cfg_if::cfg_if! {
    if #[cfg(test)] {
        lazy_static::lazy_static! {
            static ref COUNTER: Mutex<usize> = Mutex::new(0);
        }

        async fn assert_max_proc() {
            {
                let mut count = COUNTER.lock().await;
                *count += 1;
                assert!(*count <= CONFIG.max_proc);
            }
            tokio::time::delay_for(Duration::from_secs(3)).await;
            {
                let mut count = COUNTER.lock().await;
                *count -= 1;
                assert!(*count <= CONFIG.max_proc);
            }
        }

        async fn test_python_in_container(_code: &str, _func_name: &str, _test_case: &str, _time: Duration) -> String {
            assert_max_proc();
            String::new()
        }

        async fn test_java_in_container(_code: &str, _func_name: &str, _test_case: &str, _time: Duration) -> String {
            assert_max_proc();
            String::new()
        }
    } else {
        use executor::test::test_python_in_container;
        use executor::test::test_java_in_container;
    }
}





// TODO: maybe want to reject duplicate requests that come in too quickly
//       this probably means getting a database, which is annoying but ehh
async fn run(addr:Option<SocketAddr>, language: String, identifier: String, code: bytes::Bytes) -> Result<impl warp::Reply, warp::Rejection> {
    {
        let mut unlocked_cache = CACHE.lock().await;
        let addr = match addr {
            Some(a) => a,
            None => return Ok(Response::builder().status(StatusCode::INTERNAL_SERVER_ERROR).body("Internal server error".to_owned())),
        };
        match unlocked_cache.get(&addr) {
            Some(t) => {
                if t.elapsed() < *TIMEOUT {
                    return Ok(Response::builder().status(StatusCode::OK).body(
                        json!({"sucess": false, "msg": "Code ran too soon. Please wait a little."}).to_string()
                    ))
                 }
            },
            None => {}
        };
        unlocked_cache.put(addr.to_owned(), Instant::now());
    }

    {
        let _sema = BOUND.acquire().await;
        let decoded_code = match from_utf8(&code) {
            Ok(c) => c,
            Err(_) => return Ok(Response::builder().status(StatusCode::BAD_REQUEST).body(String::new())),
        };

        match (&CONFIG.questions.get(&identifier), &language[..]) {
            (None, _) => Err(warp::reject::reject()),
            (Some(q), "python") => {
                let resp = test_python_in_container(
                    decoded_code,
                    &q.function_name,
                    &q.python.test_case,
                    *TIMEOUT
                ).await;
                trace!(target: "Run", "Got python code {}. Sent response {} to {}", decoded_code, resp, addr.unwrap());
                Ok(Response::builder().body(resp))
            },
            (Some(q), "java") => {
                let resp = test_java_in_container(
                    decoded_code,
                    &CONFIG.java_test_file,
                    &q.java.func_call,
                    &q.java.test_case,
                    *TIMEOUT
                ).await;
                trace!(target: "Run", "Got java code {}. Sent response {} to {}", decoded_code, resp, addr.unwrap());
                Ok(Response::builder().body(resp))
            },
            _ => Err(warp::reject::reject()),
        }
    }
}

fn run_filter() -> impl warp::Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::post()
        .and(warp::path("run"))
        .and(warp::addr::remote())
        .and(warp::path::param::<String>())
        .and(warp::path::param::<String>())
        .and(warp::body::content_length_limit(1024 * 64))
        .and(warp::body::bytes())
        .and_then(run)
        .with(
            // Probably want to make this not *
            warp::reply::with::default_header("Access-Control-Allow-Origin", "*")
        )
}

async fn get_boilerplate(q: String, s: String) -> Result<impl warp::Reply, warp::Rejection> {
    CONFIG.questions.get(&q).map_or(
        Err(warp::reject::reject()),
        |q|{
            match &s[..] {
                "python" => Ok(q.python.boilerplate.to_owned()),
                "java" => Ok(q.java.boilerplate.to_owned()),
                _ => Err(warp::reject::reject()),
            }
        }
    )
}


#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    // We want to crash early if there's something wrong with the configs
    verify_config();
    &STATIC_CONTENT[..];

    // TODO: Config this
    // But theres also no way we get 100 simultaneous connections every 2 seconds

    let run = run_filter();
    // TODO: Maybe this can be static instead of doing this arc bs
    let page = warp::get()
        .and(warp::path::end())
        .map(|| warp::reply::html(STATIC_CONTENT.as_bytes()));

    let get_boilerplate = warp::get()
        .and(warp::path("boilerplate"))
        .and(warp::path::param::<String>())
        .and(warp::path::param::<String>())
        .and_then(get_boilerplate)
        .with(
            // Probably want to make this not *
            warp::reply::with::default_header("Access-Control-Allow-Origin", "*")
        );

    let router = run.or(get_boilerplate).or(page);
    warp::serve(router).run(([127, 0, 0, 1], 8080)).await;
}

#[cfg(test)]
mod test {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};

    #[tokio::test]
    async fn test_run_spam() {
        let (handle, question) = CONFIG.questions.iter().next().unwrap();

        let val1 = warp::test::request()
            .method("POST")
            .remote_addr(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 0, 14)), 8080))
            .path(&format!("/run/{}", handle))
            .body(&format!("def {}:\n    return False", question.function_name));

        let val2 = warp::test::request()
            .method("POST")
            .remote_addr(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 0, 14)), 8080))
            .path(&format!("/run/{}", handle))
            .body(&format!("def {}:\n    return False", question.function_name));

        let res1 = tokio::spawn(async move {
            let filter = run_filter();
            val1.reply(&filter).await
            }
        );
        let res2 = tokio::spawn(async move {
            let filter = run_filter();
            val2.reply(&filter).await
            }
        );

        assert_eq!(
            from_utf8(res2.await.unwrap().body()).unwrap(),
            json!({"sucess": false, "msg": "Code ran too soon. Please wait a little."}).to_string()
        );

        res1.await.unwrap();
    }

    #[tokio::test]
    async fn test_run_max_req() {

        let mut vals = Vec::new();

        for i in 0..CONFIG.max_proc + 1 {
            vals.push(
                tokio::spawn(
                    async move {
                        let filter = run_filter();
                        let (handle, question) = CONFIG.questions.iter().next().unwrap();
                        warp::test::request()
                        .method("POST")
                        .remote_addr(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 0, i as u8)), 8080))
                        .path(&format!("/run/{}", handle))
                        .body(&format!("def {}:\n    import time\n    sleep(40)", question.function_name))
                        .reply(&filter).await
                    }
                )
            )
        }

        for v in vals {
            v.await.unwrap();
        }
    }
}
