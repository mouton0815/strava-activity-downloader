use std::error::Error;
use log::{debug, info};
use oauth2::basic::BasicClient;
use oauth2::{AuthorizationCode, AuthType, AuthUrl, ClientId, ClientSecret, CsrfToken, HttpRequest, HttpResponse, RedirectUrl, ResourceOwnerPassword, ResourceOwnerUsername, Scope, TokenResponse, TokenUrl};
use oauth2::reqwest::async_http_client;
use url::Url;
use crate::oauth::token;
use crate::TokenHolder;

pub const AUTH_CALLBACK : &'static str = "/auth_callback";

type TokenResult = Result<TokenHolder, Box<dyn Error>>;

pub struct OAuthClient {
    client: BasicClient,
    scopes: Vec<String>
}

impl OAuthClient {
    pub fn new(host: &str,
               port: u64,
               client_id: String,
               client_secret: String,
               auth_url: String,
               token_url: String,
               scopes: Vec<String>,
    ) -> Result<Self, Box<dyn Error>> {
        let redirect_url = format!("http://{}:{}{}", host, port, AUTH_CALLBACK);
        let client = BasicClient::new(
            ClientId::new(client_id),
            Some(ClientSecret::new(client_secret.to_string())),
            AuthUrl::new(auth_url.to_string())?,
            Some(TokenUrl::new(token_url.to_string())?)
        ).set_redirect_uri(RedirectUrl::new(redirect_url)?)
            .set_auth_type(AuthType::RequestBody);
        Ok(Self { client, scopes })
    }

    #[allow(dead_code)]
    pub async fn authorize_password_grant(&self, user: &str, pass: &str) -> TokenResult {
        let token = self.client
            .exchange_password(
                &ResourceOwnerUsername::new(user.to_string()),
                &ResourceOwnerPassword::new(pass.to_string())
            )
            .request_async(async_http_client)
            .await?;

        TokenHolder::new(token)
    }

    pub fn authorize_auth_code_grant(&self) -> Result<(Url, CsrfToken), Box<dyn Error>> {
        // Transform Vec<String> to Vec<Scope>.
        // Note that cloning is needed anyway because Client.add_scopes() moves its argument.
        let scopes : Vec<Scope> = self.scopes.iter().map(|s| Scope::new(s.clone())).collect();
        let (auth_url, csrf_token) = self.client
            .authorize_url(CsrfToken::new_random)
            .add_scopes(scopes.into_iter())
            .url();
        debug!("State is {}", csrf_token.secret());
        Ok((auth_url, csrf_token))
    }

    pub async fn exchange_code_for_token(&self, code: &String) -> TokenResult {
        debug!("Obtain token for code {}", code);
        let token = token::validate(self.client
            .exchange_code(AuthorizationCode::new(code.clone()))
            .request_async(request_wrapper)
            .await?)?;

        info!("Obtained token");
        debug!("{:?}", token);

        TokenHolder::new(token)
    }

    pub async fn refresh_token(&self, token_holder: &TokenHolder) -> TokenResult {
        debug!("Access token expired, refreshing ...");
        let token = token::validate(self.client
            .exchange_refresh_token(&token_holder.token().refresh_token().unwrap())
            .request_async(request_wrapper)
            .await?)?;

        info!("Refreshed token successfully");
        TokenHolder::new(token)
    }
}

async fn request_wrapper(request: HttpRequest) -> Result<HttpResponse, oauth2::reqwest::Error<reqwest::Error>> {
    debug!("Token request URL: {}", request.url);
    debug!("Token request body: {:?}", String::from_utf8_lossy(&request.body));
    async_http_client(request).await
}
