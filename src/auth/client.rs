use std::error::Error;
use log::{debug, info};
use oauth2::basic::BasicClient;
use oauth2::{AuthorizationCode, AuthType, AuthUrl, ClientId, ClientSecret, CsrfToken, RedirectUrl, ResourceOwnerPassword, ResourceOwnerUsername, TokenResponse, TokenUrl};
use oauth2::reqwest::async_http_client;
use url::Url;
use crate::auth::token;
use crate::TokenHolder;

const CLIENT_ID : &'static str = "unite-client";
const CLIENT_SECRET : &'static str = "totally-secret";
const AUTH_URL : &'static str = "http://localhost:8080/realms/unite/protocol/openid-connect/auth";
const TOKEN_URL : &'static str = "http://localhost:8080/realms/unite/protocol/openid-connect/token";

pub const AUTH_CALLBACK : &'static str = "/auth_callback";

type TokenResult = Result<TokenHolder, Box<dyn Error>>;

pub struct OAuthClient(BasicClient);

impl OAuthClient {
    // TODO: Pass parameters
    pub fn new(host: &str, port: &str) -> Result<Self, Box<dyn Error>> {
        let redirect_url = format!("http://{}:{}{}", host, port, AUTH_CALLBACK);
        let client = BasicClient::new(
            ClientId::new(CLIENT_ID.to_string()),
            Some(ClientSecret::new(CLIENT_SECRET.to_string())),
            AuthUrl::new(AUTH_URL.to_string())?,
            Some(TokenUrl::new(TOKEN_URL.to_string())?)
        ).set_redirect_uri(RedirectUrl::new(redirect_url)?)
            .set_auth_type(AuthType::RequestBody);
        Ok(Self { 0: client })
    }

    #[allow(dead_code)]
    pub async fn authorize_password_grant(&self, user: &str, pass: &str) -> TokenResult {
        let token = self.0
            .exchange_password(
                &ResourceOwnerUsername::new(user.to_string()),
                &ResourceOwnerPassword::new(pass.to_string())
            )
            .request_async(async_http_client)
            .await?;

        TokenHolder::new(token)
    }

    pub fn authorize_auth_code_grant(&self) -> Result<(Url, CsrfToken), Box<dyn Error>> {
        let (auth_url, csrf_token) = self.0
            .authorize_url(CsrfToken::new_random)
            .url();
        debug!("State is {}", csrf_token.secret());
        Ok((auth_url, csrf_token))
    }

    pub async fn exchange_code_for_token(&self, code: &String) -> TokenResult {
        debug!("Obtain token for code {}", code);
        let token = token::validate(self.0
            .exchange_code(AuthorizationCode::new(code.clone()))
            .request_async(async_http_client)
            .await?)?;

        info!("Obtained token");
        debug!("{:?}", token);

        TokenHolder::new(token)
    }

    pub async fn refresh_token(&self, token_holder: &TokenHolder) -> TokenResult {
        debug!("Access token expired, refreshing ...");
        let token = token::validate(self.0
            .exchange_refresh_token(&token_holder.token.refresh_token().unwrap())
            .request_async(async_http_client)
            .await?)?;

        info!("Refreshed token successfully");
        TokenHolder::new(token)
    }
}

/*
async fn request_wrapper(request: HttpRequest) -> Result<HttpResponse, oauth2::reqwest::Error<reqwest::Error>> {
    debug!("Token request URL: {}", request.url);
    debug!("Token request body: {:?}", String::from_utf8_lossy(&request.body));
    async_http_client(request).await
}
*/
