use std::error::Error;
use std::sync::{Arc, Mutex, MutexGuard};
use axum::http::StatusCode;
use axum::{Json, Router};
use axum::extract::{Query, State};
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::get;
use gethostname::gethostname;
use log::{error, info};
use oauth2::{AuthorizationCode, AuthUrl, ClientId, ClientSecret, CsrfToken, RedirectUrl, TokenResponse, TokenUrl};
use oauth2::basic::{BasicClient, BasicTokenResponse};
use oauth2::reqwest::async_http_client;
use serde::{Serialize, Deserialize};
use url::Url;

const PORT : &'static str = "3000";

const CLIENT_ID : &'static str = "unite-client";
const CLIENT_SECRET : &'static str = "totally-secret";
const AUTH_URL : &'static str = "http://localhost:8080/realms/unite/protocol/openid-connect/auth";
const TOKEN_URL : &'static str = "http://localhost:8080/realms/unite/protocol/openid-connect/token";

/*
fn authorize_password_grant() -> Result<BasicTokenResponse, Box<dyn Error>> {
    let client =
        BasicClient::new(
            ClientId::new("unite-client".to_string()),
            Some(ClientSecret::new("totally-secret".to_string())),
            AuthUrl::new("http://localhost:8080/realms/unite/protocol/openid-connect/auth".to_string())?,
            Some(TokenUrl::new("http://localhost:8080/realms/unite/protocol/openid-connect/token".to_string())?)
        );

    let token_result =
        client
            .exchange_password(
                &ResourceOwnerUsername::new("fred".to_string()),
                &ResourceOwnerPassword::new("fred".to_string())
            )
            //.add_scope(Scope::new("read".to_string()))
            .request(http_client)?;

    Ok(token_result)
}
*/

fn authorize_auth_code_grant(oauth_client: BasicClient) -> Result<Url, Box<dyn Error>> {
    let (auth_url, _csrf_token) = oauth_client
        .authorize_url(CsrfToken::new_random)
        //.set_pkce_challenge(pkce_challenge)
        .url();

    Ok(auth_url)
}

async fn exchange_code_for_token(oauth_client: BasicClient, code: String) -> Result<BasicTokenResponse, Box<dyn Error>> {
    info!("Get token with code {}", code);
    let token = oauth_client
        .exchange_code(AuthorizationCode::new(code))
        //.set_pkce_verifier(pkce_verifier)
        //.request(http_client)?;
        .request_async(async_http_client)
        .await?;

    Ok(token)
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct ErrorResult {
    error: String
}

type RetrieveResult = Result<Response, (StatusCode, Json<ErrorResult>)>;

async fn retrieve(State(state): State<SharedState>) -> RetrieveResult {
    let mut guard = state.token.lock().unwrap();
    match &mut *guard {
        Some(_token) => {
            info!("Retrieve: Token found");
            Ok("foo bar".into_response())
        }
        None => {
            info!("Retrieve: NO token, redirect");
            Ok(Redirect::temporary("/authorize").into_response())
        }
    }
}

type AuthorizeResult = Result<Redirect, (StatusCode, Json<ErrorResult>)>;

async fn authorize(State(state): State<SharedState>) -> AuthorizeResult {
    info!("Authorizing...");
    match authorize_auth_code_grant(state.oauth_client) {
        Ok(url) => {
            info!("Success: {}", url);
            Ok(Redirect::temporary(url.as_str()))
        }
        Err(error) => {
            error!("Error: {}", error);
            let message = ErrorResult{ error: error.to_string() };
            Err((StatusCode::INTERNAL_SERVER_ERROR, Json(message)))
        }
    }
}

// TODO: Accept and check state
#[derive(Deserialize)]
struct CallbackQuery {
    code: String
}

type CallbackResult = Result<Redirect, (StatusCode, Json<ErrorResult>)>;

async fn auth_callback(State(state): State<SharedState>, query: Query<CallbackQuery>) -> CallbackResult {
    // TODO: Verify signature
    info!("... authorized with code {}", query.code);
    match exchange_code_for_token(state.oauth_client, query.code.clone()).await {
        Ok(token) => {
            info!("Token: {:?}", token);
            info!("Access: {:?}", token.access_token());
            info!("Secret: {}", token.access_token().secret());
            let mut guard: MutexGuard<'_, Option<BasicTokenResponse>> = state.token.lock().unwrap();
            *guard = Some(token);
            Ok(Redirect::temporary("/retrieve")) // TODO: Should be URL take from session or parameter
        }
        Err(error) => {
            error!("Error: {}", error);
            let message = ErrorResult{ error: error.to_string() };
            Err((StatusCode::UNAUTHORIZED, Json(message)))
        }
    }
}

fn create_oauth_client() -> Result<BasicClient, Box<dyn Error>> {
    let host = gethostname();
    let redirect_url = format!("http://{}:{}/auth_callback", host.to_str().unwrap(), PORT);
    //let redirect_url = format!("http://{}:{}/auth_callback", "localhost", PORT);
    Ok(BasicClient::new(
        ClientId::new(CLIENT_ID.to_string()),
        Some(ClientSecret::new(CLIENT_SECRET.to_string())),
        AuthUrl::new(AUTH_URL.to_string())?,
        Some(TokenUrl::new(TOKEN_URL.to_string())?)
    ).set_redirect_uri(RedirectUrl::new(redirect_url)?))
}

#[derive(Clone)]
struct SharedState {
    oauth_client: BasicClient,
    token : Arc<Mutex<Option<BasicTokenResponse>>>
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>>  {
    env_logger::init();
    info!("Hello, world!");

    let shared_state = SharedState {
        oauth_client: create_oauth_client()?,
        token: Arc::new(Mutex::new(None))
    };

    let app = Router::new()
        .route("/retrieve", get(retrieve))
        .route("/authorize", get(authorize))
        .route("/auth_callback", get(auth_callback))
        .with_state(shared_state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    Ok(axum::serve(listener, app).await?)
}
