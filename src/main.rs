use std::error::Error;
use std::sync::Arc;
use axum::http::{StatusCode, Uri};
use axum::{Json, middleware, Router};
use axum::extract::{Extension, Query, Request, State};
use axum::middleware::Next;
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::get;
use axum_macros::debug_handler;
use log::{info, warn};
use oauth2::{AuthorizationCode, AuthUrl, ClientId, ClientSecret, CsrfToken, RedirectUrl, TokenResponse, TokenUrl};
use oauth2::basic::{BasicClient, BasicTokenResponse};
use oauth2::reqwest::async_http_client;
use serde::{Serialize, Deserialize};
use tokio::sync::Mutex;
use url::Url;
use crate::auth::client::OAuthClient;

mod auth;

const HOST : &'static str = "localhost";
const PORT : &'static str = "3000";

const CLIENT_ID : &'static str = "unite-client";
const CLIENT_SECRET : &'static str = "totally-secret";
const AUTH_URL : &'static str = "http://localhost:8080/realms/unite/protocol/openid-connect/auth";
const TOKEN_URL : &'static str = "http://localhost:8080/realms/unite/protocol/openid-connect/token";

const AUTH_CALLBACK : &'static str = "/auth_callback";

#[derive(Clone, Debug)]
struct Bearer(String);

impl From<String> for Bearer {
    fn from(item: String) -> Self {
        Self { 0: item }
    }
}

impl From<Bearer> for String {
    fn from(item: Bearer) -> Self {
        item.0
    }
}

fn create_oauth_client() -> Result<BasicClient, Box<dyn Error>> {
    let redirect_url = format!("http://{}:{}{}", HOST, PORT, AUTH_CALLBACK);
    Ok(BasicClient::new(
        ClientId::new(CLIENT_ID.to_string()),
        Some(ClientSecret::new(CLIENT_SECRET.to_string())),
        AuthUrl::new(AUTH_URL.to_string())?,
        Some(TokenUrl::new(TOKEN_URL.to_string())?)
    ).set_redirect_uri(RedirectUrl::new(redirect_url)?))
}

fn authorize_auth_code_grant(oauth_client: &BasicClient) -> Result<(Url, CsrfToken), Box<dyn Error>> {
    let (auth_url, csrf_token) = oauth_client
        .authorize_url(CsrfToken::new_random)
        //.set_pkce_challenge(pkce_challenge)
        .url();
    // info!("-----> state is {}", csrf_token.secret());
    Ok((auth_url, csrf_token))
}

type TokenResult = Result<(BasicTokenResponse, u64), Box<dyn Error>>;

async fn exchange_code_for_token(oauth_client: &BasicClient, code: String) -> TokenResult {
    info!("--c--> Obtain token for code {}", code); // TODO: Change to debug!()
    let token = auth::token::validate(oauth_client
        .exchange_code(AuthorizationCode::new(code))
        //.set_pkce_verifier(pkce_verifier)
        .request_async(async_http_client)
        .await?)?;

    let bearer : String = token.access_token().secret().chars().take(100).collect();
    info!("--c--> Token obtained: {}", bearer);
    let expiry = auth::token::get_expiry_time(token.access_token())?;
    Ok((token, expiry))
}

async fn refresh_token(oauth_client: &BasicClient, token: &BasicTokenResponse) -> TokenResult {
    info!("Access token expired, refreshing ...");
    let token = auth::token::validate(oauth_client
        .exchange_refresh_token(&token.refresh_token().unwrap())
        .request_async(async_http_client)
        .await?)?;

    let bearer : String = token.access_token().secret().chars().take(100).collect();
    info!("--c--> Token obtained: {}", bearer);
    let expiry = auth::token::get_expiry_time(token.access_token())?;
    Ok((token, expiry))
}

// TODO: Note: Middleware can also return Result<Response, StatusCode> (or similar?)
async fn auth_middleware(State(state): State<MutexState>, mut request: Request, next: Next) -> Response {
    info!("--m--> Request URI: {}", request.uri());
    // Do no apply middleware to auth callback route
    if request.uri().path().starts_with(AUTH_CALLBACK) {
        info!("--m--> Bypass middleware for auth callback: {}", request.uri());
        let response = next.run(request).await;
        info!("--m--> Response status: {}", response.status());
        return response;
    }
    let mut guard = state.lock().await;
    match &(*guard).token {
        Some((token, expiry)) => {
            info!("--m--> Token found");
            if auth::token::is_expired(expiry) {
                match refresh_token(&(*guard).oauth_client, token).await {
                    Ok(token) => {
                        (*guard).token = Some(token);
                        // TODO: Drop guard here?
                    }
                    Err(error) => {
                        return to_internal_server_error(error).into_response();
                    }
                }
            }
            let access = (*guard).token.as_ref().expect("Missing token").0.access_token();
            let bearer = format!("Bearer {}", access.secret());
            request.extensions_mut().insert(bearer); // TODO: Extract Bearer earlier!
            drop(guard); // Inner layers might also want to obtain the mutex
            info!("--m--> Delegate to next layer");
            let response = next.run(request).await;
            info!("--m--> Response status: {}", response.status());
            response
        }
        None => {
            info!("--m--> NO token, build authorization URL");
            match authorize_auth_code_grant(&(*guard).oauth_client) {
                Ok((url, csrf_token)) => {
                    info!("--m--> Redirect to authorization URL: {}", url);
                    (*guard).oauth_state = Some(csrf_token.secret().clone());
                    (*guard).origin = Some(request.uri().clone());
                    Redirect::temporary(url.as_str()).into_response()
                }
                Err(error) => {
                    to_internal_server_error(error).into_response()
                }
            }
        }
    }
}

#[derive(Serialize, Debug, Eq, PartialEq)]
struct ErrorResult {
    error: String
}

type RestError = (StatusCode, Json<ErrorResult>);

fn to_internal_server_error(error: Box<dyn Error>) -> RestError {
    warn!("Error: {}", error);
    let message = ErrorResult { error: error.to_string() };
    (StatusCode::INTERNAL_SERVER_ERROR, Json(message))
}

#[debug_handler]
async fn retrieve(Extension(bearer): Extension<Bearer>) -> Response {
    info!("--r--> Enter /retrieve");
    let bearer : String = bearer.into();
    info!("--r--> {}", bearer);
    // TODO: Do something useful
    Json("foo bar").into_response()
}

#[derive(Deserialize)]
struct CallbackQuery {
    code: String,
    state: String
}

async fn auth_callback(State(state): State<MutexState>, query: Query<CallbackQuery>) -> Result<Redirect, RestError> {
    info!("--c--> Authorized with code {}", query.code);
    let mut guard = state.lock().await;
    if (*guard).origin.is_none()
        || (*guard).oauth_state.is_none()
        || (*guard).oauth_state.as_ref().unwrap() != &query.state {
        warn!("Received state {} does not match expected state {}", query.state,
            (*guard).oauth_state.as_ref().unwrap_or(&String::from("<null>")));
        return Err(to_internal_server_error("Internal error".into()))
    }
    match exchange_code_for_token(&(*guard).oauth_client, query.code.clone()).await {
        Ok(token) => {
            let uri = (*guard).origin.as_ref().unwrap().to_string();
            (*guard).token = Some(token);
            (*guard).oauth_state = None;
            (*guard).origin = None;
            info!("--c--> Redirect to origin URL: {}", uri);
            Ok(Redirect::temporary(uri.as_str()))
        }
        Err(error) => {
            warn!("Error: {}", error);
            let message = ErrorResult{ error: error.to_string() };
            Err((StatusCode::UNAUTHORIZED, Json(message)))
        }
    }
}

#[derive(Clone)]
struct SharedState {
    oauth_client: BasicClient,
    oauth_state: Option<String>,
    origin: Option<Uri>, // REST endpoint that triggered the authentication
    token: Option<(BasicTokenResponse, u64)> // Extract expiry time only once and store it
}

type MutexState = Arc<Mutex<SharedState>>;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>>  {
    env_logger::init();
    let client = OAuthClient::new()?;
    let token = client.authorize_password_grant("fred", "fred").await?;
    info!("--x--> {:?}", token.refresh_token());
    auth::token::validate(token)?;

    let shared_state = Arc::new(Mutex::new(SharedState {
        oauth_client: create_oauth_client()?,
        oauth_state: None,
        origin: None,
        token: None
    }));

    let app = Router::new()
        .route("/retrieve", get(retrieve))
        .route(AUTH_CALLBACK, get(auth_callback))
        .route_layer(middleware::from_fn_with_state(shared_state.clone(), auth_middleware))
        .with_state(shared_state);

    let listener = tokio::net::TcpListener::bind(format!("{}:{}", HOST, PORT)).await?;
    Ok(axum::serve(listener, app).await?)
}
