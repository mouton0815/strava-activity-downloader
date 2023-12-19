use std::error::Error;
use std::sync::Arc;
use axum::http::StatusCode;
use axum::{Json, Router};
use axum::extract::{Query, State};
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::get;
use log::{info, warn};
use oauth2::{AuthorizationCode, AuthUrl, ClientId, ClientSecret, CsrfToken, RedirectUrl, ResourceOwnerPassword, ResourceOwnerUsername, TokenResponse, TokenUrl};
use oauth2::basic::{BasicClient, BasicTokenResponse};
use oauth2::reqwest::async_http_client;
use serde::{Serialize, Deserialize};
use tokio::sync::Mutex;
use url::Url;

mod util;

const HOST : &'static str = "localhost";
const PORT : &'static str = "3000";

const CLIENT_ID : &'static str = "unite-client";
const CLIENT_SECRET : &'static str = "totally-secret";
const AUTH_URL : &'static str = "http://localhost:8080/realms/unite/protocol/openid-connect/auth";
const TOKEN_URL : &'static str = "http://localhost:8080/realms/unite/protocol/openid-connect/token";

async fn authorize_password_grant() -> Result<BasicTokenResponse, Box<dyn Error>> {
    let client = create_oauth_client()?;
    let token_result = client
        .exchange_password(
            &ResourceOwnerUsername::new("fred".to_string()),
            &ResourceOwnerPassword::new("fred".to_string())
        )
        .request_async(async_http_client)
        .await?;

    Ok(token_result)
}

fn authorize_auth_code_grant(oauth_client: &BasicClient) -> Result<(Url, CsrfToken), Box<dyn Error>> {
    let (auth_url, csrf_token) = oauth_client
        .authorize_url(CsrfToken::new_random)
        //.set_pkce_challenge(pkce_challenge)
        .url();
    info!("----> state is {}", csrf_token.secret());
    Ok((auth_url, csrf_token))
}

type TokenResult = Result<BasicTokenResponse, Box<dyn Error>>;

async fn exchange_code_for_token(oauth_client: &BasicClient, code: String) -> TokenResult {
    info!("Obtain token for code {}", code);
    let token = oauth_client
        .exchange_code(AuthorizationCode::new(code))
        //.set_pkce_verifier(pkce_verifier)
        .request_async(async_http_client)
        .await?;

    info!("Token obtained: {}", token.access_token().secret());
    util::jwt::validate(token)
}

async fn refresh_token(oauth_client: &BasicClient, token: &BasicTokenResponse) -> TokenResult {
    info!("Access token expired, refreshing ...");
    let token = oauth_client
        .exchange_refresh_token(&token.refresh_token().unwrap())
        .request_async(async_http_client)
        .await?;

    info!("Token refreshed: {}", token.access_token().secret());
    util::jwt::validate(token)
}

// TODO: Can this be done via axum middleware?
async fn token_middleware(State(state): State<MutexState>) -> Result<Option<String>, Box<dyn Error>> {
    let mut guard = state.lock().await;
    match &(*guard).token {
        Some(token) => {
            info!("Middleware: Token found");
            if util::jwt::expired(token)? {
                let token = refresh_token(&(*guard).oauth_client, token).await?;
                (*guard).token = Some(token);
            }
            Ok(Some((*guard).token.as_ref().unwrap().access_token().secret().clone())) // TODO: clone is not nice
        }
        None => Ok(None)
    }
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
struct ErrorResult {
    error: String
}

type RestError = (StatusCode, Json<ErrorResult>);

fn to_internal_server_error(error: Box<dyn Error>) -> RestError {
    warn!("Error: {}", error);
    let message = ErrorResult { error: error.to_string() };
    (StatusCode::INTERNAL_SERVER_ERROR, Json(message))
}

// TODO: Return type could be simplified to "Response" if middleware does not return errors
async fn retrieve(State(state): State<MutexState>) -> Result<Response, RestError> {
    match token_middleware(State(state)).await.map_err(to_internal_server_error)? {
        Some(_token) => {
            // TODO: Do something useful
            Ok(Json("foo bar").into_response())
        }
        None => {
            info!("Retrieve: NO token, redirect");
            // TODO: Put "/retrieve" into state for later redirect after authentication
            Ok(Redirect::temporary("/authorize").into_response())
        }
    }
}

async fn authorize(State(state): State<MutexState>) -> Result<Redirect, RestError> {
    info!("Authorizing...");
    let mut guard = state.lock().await;
    match authorize_auth_code_grant(&(*guard).oauth_client) {
        Ok((url, csrf_token)) => {
            info!("Success: {}", url);
            (*guard).oauth_state = Some(csrf_token.secret().clone());
            Ok(Redirect::temporary(url.as_str()))
        }
        Err(error) => Err(to_internal_server_error(error))
    }
}

#[derive(Deserialize)]
struct CallbackQuery {
    code: String,
    state: String
}

async fn auth_callback(State(state): State<MutexState>, query: Query<CallbackQuery>) -> Result<Redirect, RestError> {
    info!("... authorized with code {}", query.code);
    let mut guard = state.lock().await;
    if (*guard).oauth_state == None || (*guard).oauth_state.as_ref().unwrap() != &query.state {
        warn!("Received state {} does not match expected state {}", query.state,
            (*guard).oauth_state.as_ref().unwrap_or(&String::from("<null>")));
        return Err(to_internal_server_error("Internal error".into()))
    }
    match exchange_code_for_token(&(*guard).oauth_client, query.code.clone()).await {
        Ok(token) => {
            (*guard).token = Some(token);
            (*guard).oauth_state = None;
            Ok(Redirect::temporary("/retrieve")) // TODO: Should be URL take from session or parameter
        }
        Err(error) => {
            warn!("Error: {}", error);
            let message = ErrorResult{ error: error.to_string() };
            Err((StatusCode::UNAUTHORIZED, Json(message)))
        }
    }
}

fn create_oauth_client() -> Result<BasicClient, Box<dyn Error>> {
    let redirect_url = format!("http://{}:{}/auth_callback", HOST, PORT);
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
    oauth_state: Option<String>,
    token: Option<BasicTokenResponse>
}

type MutexState = Arc<Mutex<SharedState>>;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>>  {
    env_logger::init();
    info!("Hello, world!");
    let token = authorize_password_grant().await?;
    info!("--x--> {:?}", token.refresh_token());
    util::jwt::validate(token)?;
    info!("Hello is done");

    let shared_state = Arc::new(Mutex::new(SharedState {
        oauth_client: create_oauth_client()?,
        oauth_state: None,
        token: None
    }));

    let app = Router::new()
        .route("/retrieve", get(retrieve))
        .route("/authorize", get(authorize))
        .route("/auth_callback", get(auth_callback))
        .with_state(shared_state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    Ok(axum::serve(listener, app).await?)
}
