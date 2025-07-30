// Send is necessary to send errors between threads (needed by axum middleware):
// https://users.rust-lang.org/t/axum-middleware-trait-bound-issue-when-invoking-a-function-returning-boxed-error-result/100052/4
// Sync is necessary for From/Into convenience:
// https://users.rust-lang.org/t/convert-box-dyn-error-to-box-dyn-error-send/48856
use axum::BoxError;
use log::{debug, info, warn};
use oauth2::basic::BasicClient;
use oauth2::{AuthorizationCode, AuthType, AuthUrl, ClientId, ClientSecret, CsrfToken, HttpRequest, HttpResponse, RedirectUrl, ResourceOwnerPassword, ResourceOwnerUsername, Scope, TokenResponse, TokenUrl};
use oauth2::reqwest::async_http_client;
use url::Url;
use crate::oauth::token;
use crate::oauth::token::{Bearer, TokenHolder};

type TokenResult = Result<TokenHolder, BoxError>;
type UrlResult = Result<String, BoxError>;
type BearerResult = Result<Option<Bearer>, BoxError>;

pub struct OAuthClient {
    client: BasicClient,
    scopes: Vec<String>,
    state: Option<String>,
    target: String, // URL to be redirected to after authentication. Can be relative or absolute.
    token: Option<TokenHolder>,
}

impl OAuthClient {
    pub fn new(client_id: String,
               client_secret: String,
               auth_url: String,
               token_url: String,
               target_url: String,
               redirect_url: String,
               scopes: Vec<String>,
    ) -> Result<OAuthClient, BoxError> {
        let client = BasicClient::new(
            ClientId::new(client_id),
            Some(ClientSecret::new(client_secret.to_string())),
            AuthUrl::new(auth_url.to_string())?,
            Some(TokenUrl::new(token_url.to_string())?)
        ).set_redirect_uri(RedirectUrl::new(redirect_url)?)
            .set_auth_type(AuthType::RequestBody);

        Ok(Self { client, scopes, state: None, target: target_url, token: None })
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

        Ok(TokenHolder::new(token))
    }

    pub fn authorize_auth_code_grant(&mut self) -> Url {
        // Transform Vec<String> to Vec<Scope>.
        // Note that cloning is needed anyway because Client.add_scopes() moves its argument.
        let scopes : Vec<Scope> = self.scopes.iter().map(|s| Scope::new(s.clone())).collect();
        let (auth_url, csrf_token) = self.client
            .authorize_url(CsrfToken::new_random)
            .add_scopes(scopes)
            .url();
        self.state = Some(csrf_token.secret().clone());
        auth_url
    }

    pub async fn callback_auth_code_grant(&mut self, code: &str, state: &str) -> UrlResult {
        debug!("Authorized with code {}", code);
        if self.state.is_none() || self.state.as_ref().unwrap() != state {
            warn!("Received state {} does not match expected state {}", state,
            self.state.as_ref().unwrap_or(&String::from("<null>")));
            return Err("OAuth state does not match".into());
        }
        match self.exchange_code_for_token(code).await {
            Ok(token) => {
                self.token = Some(token);
                self.state = None;
                Ok(self.target.clone())
            }
            Err(error) => {
                warn!("Error: {:?}", error);
                Err(error)
            }
        }
    }

    async fn exchange_code_for_token(&self, code: &str) -> TokenResult {
        debug!("Obtain token for code {}", code);
        let token = token::validate(self.client
            .exchange_code(AuthorizationCode::new(code.to_string()))
            .request_async(request_wrapper)
            .await?)?;

        info!("Obtained token");
        debug!("{:?}", token);
        debug!("BEARER {}", token.access_token().secret());

        Ok(TokenHolder::new(token))
    }

    // TODO: Documentation
    // https://users.rust-lang.org/t/axum-middleware-trait-bound-issue-when-invoking-a-function-returning-boxed-error-result/100052/5
    pub async fn get_bearer(&mut self) -> BearerResult {
        // Ok(Some("xyz".to_string().into()))
        match self.token.as_ref() {
            Some(token_holder) => {
                if token::is_expired(token_holder) {
                    match self.refresh_token(token_holder).await {
                        Ok(token) => {
                            self.token = Some(token);
                        }
                        Err(error) => {
                            warn!("Error: {}", error);
                            return Err(error);
                        }
                    }
                }
                let bearer = self.token.as_ref().expect("Missing token").bearer();
                Ok(Some(bearer.clone()))
            }
            None => {
                Ok(None)
            }
        }
    }

    async fn refresh_token(&self, token_holder: &TokenHolder) -> TokenResult {
        debug!("Access token expired, refreshing ...");
        let token = token::validate(self.client
            .exchange_refresh_token(token_holder.token().refresh_token().unwrap())
            .request_async(request_wrapper)
            .await?)?;

        info!("Refreshed token successfully");
        Ok(TokenHolder::new(token))
    }
}

async fn request_wrapper(request: HttpRequest) -> Result<HttpResponse, oauth2::reqwest::Error<reqwest::Error>> {
    debug!("Token request URL: {}", request.url);
    debug!("Token request body: {:?}", String::from_utf8_lossy(&request.body));
    async_http_client(request).await
}
