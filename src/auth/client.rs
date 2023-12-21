use std::error::Error;
use oauth2::basic::{BasicClient, BasicTokenResponse};
use oauth2::{AuthUrl, ClientId, ClientSecret, RedirectUrl, ResourceOwnerPassword, ResourceOwnerUsername, TokenUrl};
use oauth2::reqwest::async_http_client;

const HOST : &'static str = "localhost";
const PORT : &'static str = "3000";

const CLIENT_ID : &'static str = "unite-client";
const CLIENT_SECRET : &'static str = "totally-secret";
const AUTH_URL : &'static str = "http://localhost:8080/realms/unite/protocol/openid-connect/auth";
const TOKEN_URL : &'static str = "http://localhost:8080/realms/unite/protocol/openid-connect/token";

const AUTH_CALLBACK : &'static str = "/auth_callback";

pub struct OAuthClient(BasicClient);

impl OAuthClient {
    // TODO: Pass parameters
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let redirect_url = format!("http://{}:{}{}", HOST, PORT, AUTH_CALLBACK);
        let client = BasicClient::new(
            ClientId::new(CLIENT_ID.to_string()),
            Some(ClientSecret::new(CLIENT_SECRET.to_string())),
            AuthUrl::new(AUTH_URL.to_string())?,
            Some(TokenUrl::new(TOKEN_URL.to_string())?)
        ).set_redirect_uri(RedirectUrl::new(redirect_url)?);
        Ok(Self { 0: client })
    }

    pub async fn authorize_password_grant(&self, user: &str, pass: &str) -> Result<BasicTokenResponse, Box<dyn Error>> {
        let token_result = self.0
            .exchange_password(
                &ResourceOwnerUsername::new(user.to_string()),
                &ResourceOwnerPassword::new(pass.to_string())
            )
            .request_async(async_http_client)
            .await?;

        Ok(token_result)
    }
}
