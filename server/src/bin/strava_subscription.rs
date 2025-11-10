use std::net::SocketAddr;
use axum::extract::Query;
use axum::http::{Method, StatusCode};
use axum::response::IntoResponse;
use axum::{BoxError, Json, Router};
use axum::routing::get;
use axum_macros::debug_handler;
use config::{Config, File};
use log::info;
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;
use tokio::task;
use tower::ServiceBuilder;
use tower_http::cors::{Any, CorsLayer};

const CONFIG_YAML : &str = "conf/application.yaml";

const SUBS_CALLBACK : &str = "/subs-callback";
const VERIFY_TOKEN : &str = "activity_downloader_123";


#[derive(Debug, Deserialize)]
pub struct StravaSubscription {
    pub id: i64,
    pub resource_state: Option<i32>,
    pub application_id: Option<i64>,
    pub callback_url: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Deserialize)]
struct HubChallenge {
    #[serde(rename = "hub.mode")]
    mode: Option<String>,
    #[serde(rename = "hub.verify_token")]
    verify_token: Option<String>,
    #[serde(rename = "hub.challenge")]
    challenge: Option<String>,
}

#[derive(Serialize)]
struct HubResponse<'a> {
    #[serde(rename = "hub.challenge")]
    challenge: &'a str,
}

#[tokio::main]
async fn main() {
    env_logger::init();

    let client_id = "80058";
    let client_secret = "76507769e096eefbdcc3914788fb21f39e15d5ed";
    let callback_url = format!("https://upset-areas-sin.loca.lt{SUBS_CALLBACK}");

    let config = Config::builder()
        .add_source(File::with_name(CONFIG_YAML))
        .build()
        .unwrap();

    let strava_url = config.get_string("strava.api_url")
        .unwrap_or("https://www.strava.com/api/v3".to_string());
    let push_subs_url = format!("{strava_url}/push_subscriptions");

    let cors = CorsLayer::new()
        .allow_methods([Method::GET])
        .allow_origin(Any);

    let router = Router::new()
        .route("/hello", get(hello_handler))
        .route(SUBS_CALLBACK, get(subs_callback_handler))
        .layer(ServiceBuilder::new().layer(cors));

    let addr = SocketAddr::from(([0, 0, 0, 0], 2525));
    let listener = TcpListener::bind(addr).await.unwrap();
    let server_handle = task::spawn(async move {
        info!("Subscription server listening on https://{addr}{SUBS_CALLBACK}");
        axum::serve(listener, router).await.unwrap();
    });

    println!("-----> PASS");

    let subscriptions = get_subscriptions(push_subs_url.as_str(), client_id, client_secret).await.unwrap();
    println!("--- HAVE SUBS ---> {subscriptions:#?}");

    if subscriptions.is_empty() {
        // Create a new subscription
        post_subscription(push_subs_url.as_str(), client_id, client_secret, callback_url.as_str()).await.unwrap();
    }

    server_handle.await.unwrap();
}

// Get the current subscriptions (if any)
async fn get_subscriptions(push_subs_url: &str, client_id: &str, client_secret: &str) -> Result<Vec<StravaSubscription>, BoxError> {
    info!("📥 GET subscriptions from {push_subs_url}");
    let response = reqwest::Client::new()
        .get(push_subs_url)
        .query(&[("client_id", client_id), ("client_secret", client_secret)])
        .send()
        .await?;

    println!("GET Status: {}", response.status());

    Ok(response.json::<Vec<StravaSubscription>>().await?)
}

async fn post_subscription(push_subs_url: &str, client_id: &str, client_secret: &str, callback_url: &str) -> Result<(), BoxError> {
    info!("📥 POST subscription to {push_subs_url} with callback URL {callback_url}");
    let response = reqwest::Client::new()
        .post(push_subs_url)
        .form(&[
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("callback_url", callback_url),
            ("verify_token", VERIFY_TOKEN)
        ])
        .send()
        .await?;

    println!("POST Status: {}", response.status());

    let status = response.status();
    if !status.is_success() {
        eprintln!("❌ Failed to create subscription: {status}");
        return Err(format!("{status}").into()) // TODO: Strange
    }
    let subscription= response.json::<StravaSubscription>().await.unwrap();
    println!("✅ Subscription created successfully: {subscription:#?}");
    Ok(())
}

#[debug_handler]
async fn subs_callback_handler(Query(params): Query<HubChallenge>) -> impl IntoResponse {
    info!("📥 Enter {SUBS_CALLBACK}");
    if let (Some(mode), Some(token), Some(challenge)) = (&params.mode, &params.verify_token, &params.challenge) {
        if mode == "subscribe" && token == VERIFY_TOKEN {
            println!("✅ Verification succeeded!");
            let body = HubResponse { challenge };
            return (StatusCode::OK, Json(body)).into_response();
        }
    }
    println!("❌ Verification failed");
    (StatusCode::BAD_REQUEST, "Invalid verification").into_response()
}

#[debug_handler]
async fn hello_handler() -> impl IntoResponse {
    info!("📥 Enter /hello");
    (StatusCode::OK, "Hurray").into_response()
}