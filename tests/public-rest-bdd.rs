use std::env;
use async_trait::async_trait;
use cucumber::{then, when, World, WorldInit};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use std::{convert::Infallible};
use chrono::prelude::*;

const SERVER_TIME_PATH: &str = "/0/public/Time";

#[derive(Serialize, Deserialize, Debug, Clone)]
struct ServerTimeResult {
    unixtime: u64,
    rfc1123: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct ServerTimeResponse {
    error: Vec<String>,
    result: ServerTimeResult,
}

// `World` is your shared, likely mutable state.
#[derive(Debug, WorldInit)]
pub struct RestWorld {
    headers: Option<reqwest::header::HeaderMap>,
    body: Option<ServerTimeResponse>,
    status: Option<reqwest::StatusCode>,
}

// `World` needs to be implemented, so Cucumber knows how to construct it
// for each scenario.
#[async_trait(?Send)]
impl World for RestWorld {
    // We do require some error type.
    type Error = Infallible;

    async fn new() -> Result<Self, Infallible> {
        Ok(Self {
            headers: None,
            body: None,
            status: None,
        })
    }
}

#[when("the server time is requested")]
async fn request_server_time(world: &mut RestWorld) {
    let url = reqwest::Url::parse(&env::var("API_URL").unwrap()).unwrap();
    let result = reqwest::get( url.join(SERVER_TIME_PATH).unwrap().as_str()).await;
    println!("{:?}", result);
    match result {
        Ok(response) => {
            world.status = Some(response.status());
            world.headers = Some(response.headers().clone());
            match &response.json::<ServerTimeResponse>().await {
                Ok(body) => {
                    let t_body = body.clone();
                    world.body = Some(t_body);
                }
                Err(err) => panic!("Error encountered could not deseriliza result: {:?}", err),
            };
        }
        Err(err) => panic!("Error encountered: {:?}", err),
    }
}

#[then("a valid JSON response is retunred")]
async fn valid_response(world: &mut RestWorld) {
    match &world.status {
        Some(_x) => {
            assert!(*_x == reqwest::StatusCode::OK, "Non 200 respoince");
        }
        None => panic!("Expected headers, found: {:?}", world.status),
    }
    match &world.headers {
        Some(_x) => {
            assert!(_x.contains_key(reqwest::header::CONTENT_TYPE));
            assert_eq!(
                _x[reqwest::header::CONTENT_TYPE],
                "application/json; charset=utf-8"
            );
        }
        None => panic!("Expected headers, found: {:?}", world.headers),
    }
    match &world.body {
        Some(_x) => {}
        None => panic!("Expected headers, found: {:?}", world.body),
    }
}

#[then("the response is not cached")]
async fn response_not_cached(world: &mut RestWorld) {
    match &world.headers {
        Some(_x) => {
            assert!(_x.contains_key(reqwest::header::CACHE_CONTROL));
            let cc = _x[reqwest::header::CACHE_CONTROL].to_str().unwrap();
            let re = Regex::new(r".*no-cache.*").unwrap();
            assert!(re.is_match(cc));

            assert!(_x.contains_key("CF-Cache-Status"));
            assert_eq!(_x["CF-Cache-Status"], "MISS");
        }
        None => panic!("Expected headers, found: {:?}", world.headers),
    }
}

#[then("the response has no error messages")]
async fn response_empty_errors(world: &mut RestWorld) {
    match &world.body {
        Some(_x) => {
            assert_eq!(_x.error.len(), 0);
        }
        None => panic!("Expected body, found: {:?}", world.headers),
    }
}

#[then(regex = r"^the system time is in a margin of (\d+) sec$")]
async fn hungry_cat(world: &mut RestWorld, state: u64) {
    match &world.body {
        Some(_x) => {
            let start = SystemTime::now();
            let since_the_epoch = start
                .duration_since(UNIX_EPOCH)
                .expect("Time went backwards");
            println!("{:?}", since_the_epoch);
            let in_ms = since_the_epoch.as_secs();
            let delta;
            if _x.result.unixtime >= in_ms {
                delta = _x.result.unixtime - in_ms;
            } else {
                delta = in_ms - _x.result.unixtime;
            }

            assert!(delta <= state, "{:?} {:?} : {:?} <= {:?}",in_ms , _x.result.unixtime, delta, state);
        }
        None => panic!("Expected body, found: {:?}", world.headers),
    }
}


#[then("the unixtime field corresponds with the rfc1123")]
async fn match_unix_rfc1123(world: &mut RestWorld) {
    match &world.body {
        Some(_x) => {
            let date = Utc.datetime_from_str(&_x.result.rfc1123, "%a, %d %b %y %H:%M:%S %z").unwrap();
            assert_eq!(_x.result.unixtime, date.timestamp() as u64);
        }
        None => panic!("Expected body, found: {:?}", world.headers),
    }
}

#[tokio::main]
async fn main() {
    let api_url = env::var("API_URL").expect("Expected env variable API_URL");
    reqwest::Url::parse(&api_url).expect("Could not parse API_URL");

    RestWorld::run("tests/features/public-rest-api.feature").await;
}
