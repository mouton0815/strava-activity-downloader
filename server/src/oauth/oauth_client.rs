use axum::BoxError;
use log::{debug, info, warn};
use oauth2::basic::{BasicClient, BasicErrorResponse, BasicRevocationErrorResponse, BasicTokenIntrospectionResponse, BasicTokenResponse};
use oauth2::{AuthorizationCode, AuthType, AuthUrl, ClientId, ClientSecret, CsrfToken, RedirectUrl, Scope, TokenResponse, TokenUrl, Client, StandardRevocableToken, EndpointSet, EndpointNotSet, ResourceOwnerUsername, ResourceOwnerPassword};
use oauth2::reqwest;
use url::Url;
use crate::oauth::token;
use crate::oauth::token::{Bearer, TokenHolder};

// About type BoxError = Box<dyn std::error::Error + Send + Sync>:
// Send is necessary to send errors between threads (needed by axum middleware):
// https://users.rust-lang.org/t/axum-middleware-trait-bound-issue-when-invoking-a-function-returning-boxed-error-result/100052/5
// Sync is necessary for From/Into convenience:
// https://users.rust-lang.org/t/convert-box-dyn-error-to-box-dyn-error-send/48856

type TokenResult = Result<TokenHolder, BoxError>;
type UrlResult = Result<String, BoxError>;
type BearerResult = Result<Option<Bearer>, BoxError>;

/// An OAuth client for the authorization of a *single* user.
/// Configures a [BasicClient] for the given URLs.
/// Keeps track on the state and the token, once obtained.
pub struct OAuthClient {
    // This extreme ugliness follows https://github.com/ramosbugs/oauth2-rs/blob/main/UPGRADE.md:
    client: Client<BasicErrorResponse, BasicTokenResponse, BasicTokenIntrospectionResponse, StandardRevocableToken, BasicRevocationErrorResponse, EndpointSet, EndpointNotSet, EndpointNotSet, EndpointNotSet, EndpointSet>,
    scopes: Vec<String>,
    target: String, // URL to be redirected to after authentication – can be relative or absolute
    state: Option<String>, // Holds the state between an auth-code request and entering the callback
    token: Option<TokenHolder> // Holds the token issued by the IdP
}

impl OAuthClient {
    pub fn new(client_id: String,
               client_secret: String,
               auth_url: String,
               token_url: String,
               target_url: String,
               redirect_url: String,
               scopes: Vec<String>,
    ) -> Self {
        let client = BasicClient::new(ClientId::new(client_id))
            .set_client_secret(ClientSecret::new(client_secret.to_string()))
            .set_auth_uri(AuthUrl::new(auth_url.to_string()).unwrap()) // Panic accepted
            .set_token_uri(TokenUrl::new(token_url.to_string()).unwrap())
            .set_redirect_uri(RedirectUrl::new(redirect_url).unwrap())
            .set_auth_type(AuthType::RequestBody);

        Self { client, scopes, target: target_url, state: None, token: None }
    }

    #[allow(dead_code)]
    pub async fn authorize_password_grant(&self, user: &str, pass: &str) -> TokenResult {
        let http_client = reqwest::ClientBuilder::new()
            .redirect(reqwest::redirect::Policy::none())
            .build()?;

        let token = token::validate(self.client
            .exchange_password(
                &ResourceOwnerUsername::new(user.to_string()),
                &ResourceOwnerPassword::new(pass.to_string())
            )
            .request_async(&http_client)
            .await?)?;

        Ok(TokenHolder::new(token))
    }

    /// Constructs and returns the authorization URL for the IdP.
    /// The caller should then redirect to that URL to start the auth-code flow.
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

    /// A function to be called in the callback handler for the auth-code flow.
    /// The function exchanges the passed auth code by a token by calling the IdP.
    /// It returns the target URL (passed to the constructor of this class)
    /// to be redirected after success.
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

        let http_client = reqwest::ClientBuilder::new()
            .redirect(reqwest::redirect::Policy::none())
            .build()?;

        let token = token::validate(self.client
            .exchange_code(AuthorizationCode::new(code.to_string()))
            .request_async(&http_client)
            .await?)?;

        info!("Obtained token");
        debug!("{:?}", token);
        debug!("BEARER {}", token.access_token().secret());

        Ok(TokenHolder::new(token))
    }

    /// Returns the previously obtained token or [None] if none was acquired so far.
    /// It the token is expired, it is refreshed before returning.
    pub async fn get_bearer(&mut self) -> BearerResult {
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

        let http_client = reqwest::ClientBuilder::new()
            .redirect(reqwest::redirect::Policy::none())
            .build()?;

        let token = token::validate(self.client
            .exchange_refresh_token(token_holder.token().refresh_token().unwrap())
            .request_async(&http_client)
            .await?)?;

        info!("Refreshed token successfully");
        Ok(TokenHolder::new(token))
    }
}

#[cfg(test)]
mod tests {
    use oauth2::basic::BasicClient;
    use oauth2::{AuthUrl, ClientId, ClientSecret, TokenUrl};
    use crate::oauth::oauth_client::OAuthClient;
    use wiremock::{MockServer, Mock, ResponseTemplate};
    use wiremock::matchers::{method, path, body_string_contains};
    use serde_json::json;
    use urlencoding::encode;

    impl OAuthClient {
        pub fn dummy() -> Self {
            let dummy_url = "https://dummy.org";
            let client = BasicClient::new(ClientId::new("dummy-client".to_string()))
                .set_client_secret(ClientSecret::new("dummy-secret".to_string()))
                .set_auth_uri(AuthUrl::new(dummy_url.to_string()).unwrap())
                .set_token_uri(TokenUrl::new(dummy_url.to_string()).unwrap());

            Self { client, scopes: vec![], state: None, target: dummy_url.to_string(), token: None }
        }

        // Test helper to inspect state
        fn get_state(&self) -> Option<&String> {
            self.state.as_ref()
        }

        // Test helper to check if token exists
        fn has_token(&self) -> bool {
            self.token.is_some()
        }
    }

    fn create_mock_token_response(include_refresh: bool) -> serde_json::Value {
        let mut response = json!({
            "access_token": "mock_access_token_12345",
            "token_type": "Bearer",
            "expires_in": 3600,
            "scope": "read write"
        });
        
        if include_refresh {
            response["refresh_token"] = json!("mock_refresh_token_67890");
        }
        
        response
    }

    fn create_mock_client(mock_server: &MockServer) -> OAuthClient {
        OAuthClient::new(
            "test-client".to_string(),
            "test-secret".to_string(),
            format!("{}/authorize", mock_server.uri()),
            format!("{}/token", mock_server.uri()),
            "/dashboard".to_string(),
            format!("{}/callback", mock_server.uri()),
            vec!["read".to_string(), "write".to_string()],
        )
    }

    #[tokio::test]
    async fn test_password_grant() {
        let mock_server = MockServer::start().await;
        
        Mock::given(method("POST"))
            .and(path("/token"))
            .and(body_string_contains("grant_type=password"))
            .and(body_string_contains("username=testuser"))
            .and(body_string_contains("password=testpass"))
            .respond_with(ResponseTemplate::new(200).set_body_json(create_mock_token_response(true)))
            .mount(&mock_server)
            .await;

        let client = create_mock_client(&mock_server);
        let result = client.authorize_password_grant("testuser", "testpass").await;
        
        assert!(result.is_ok());
        let token_holder = result.unwrap();
        let bearer_string: String = token_holder.bearer().clone().into();
        assert!(bearer_string.contains("mock_access_token_12345"));
    }

    #[tokio::test]
    async fn test_auth_code_flow() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/token"))
            .and(body_string_contains("grant_type=authorization_code"))
            .respond_with(ResponseTemplate::new(200).set_body_json(create_mock_token_response(true)))
            .mount(&mock_server)
            .await;

        let mut client = create_mock_client(&mock_server);

        // Step 1: Generate and verify authorization URL
        let auth_url = client.authorize_auth_code_grant();
        assert!(auth_url.as_str().starts_with(mock_server.uri().as_str()));
        assert_eq!(auth_url.path(), "/authorize");

        let callback = format!("{}/callback", mock_server.uri().as_str());
        let callback = encode(callback.as_str());
        assert!(auth_url.as_str().contains(callback.as_ref()));

        let params: std::collections::HashMap<_, _> = auth_url.query_pairs().collect();
        assert_eq!(params.get("client_id").map(|s| s.as_ref()), Some("test-client"));
        assert_eq!(params.get("response_type").map(|s| s.as_ref()), Some("code"));
        assert!(params.contains_key("state"));

        let scope = params.get("scope").map(|s| s.as_ref()).unwrap_or("");
        assert!(scope.contains("read"));
        assert!(scope.contains("write"));

        // Step 2: Simulate callback with auth code
        let state = client.get_state().unwrap().clone();
        let redirect_url = client.callback_auth_code_grant("auth_code_xyz", &state).await;
        assert!(redirect_url.is_ok());
        assert_eq!(redirect_url.unwrap(), "/dashboard");

        // Step 3: Get bearer token
        assert!(client.has_token()); // Should already be there (no refresh needed)
        let bearer = client.get_bearer().await;
        assert!(bearer.is_ok());
        assert!(bearer.unwrap().is_some());
    }

    #[tokio::test]
    async fn test_token_exchange_fails() {
        let mock_server = MockServer::start().await;
        
        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(ResponseTemplate::new(401).set_body_json(json!({
                "error": "invalid_grant",
                "error_description": "Authorization code is invalid"
            })))
            .mount(&mock_server)
            .await;

        let mut client = create_mock_client(&mock_server);
        let _auth_url = client.authorize_auth_code_grant();
        let state = client.get_state().unwrap().clone();

        let result = client.callback_auth_code_grant("invalid_code", &state).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_token_refresh() {
        let mock_server = MockServer::start().await;
        
        // First token with very short expiry (1 second)
        let short_expiry_token = json!({
            "access_token": "old_access_token",
            "token_type": "Bearer",
            "expires_in": 1,
            "refresh_token": "mock_refresh_token_67890",
            "scope": "read"
        });
        
        // Refreshed token
        let refreshed_token = json!({
            "access_token": "new_access_token",
            "token_type": "Bearer",
            "expires_in": 3600,
            "refresh_token": "new_refresh_token",
            "scope": "read"
        });

        // Initial token grant via auth code
        Mock::given(method("POST"))
            .and(path("/token"))
            .and(body_string_contains("grant_type=authorization_code"))
            .respond_with(ResponseTemplate::new(200).set_body_json(short_expiry_token))
            .mount(&mock_server)
            .await;

        // Token refresh
        Mock::given(method("POST"))
            .and(path("/token"))
            .and(body_string_contains("grant_type=refresh_token"))
            .and(body_string_contains("refresh_token=mock_refresh_token_67890"))
            .respond_with(ResponseTemplate::new(200).set_body_json(refreshed_token))
            .mount(&mock_server)
            .await;

        let mut client = create_mock_client(&mock_server);

        // Get initial token via auth code flow
        let _auth_url = client.authorize_auth_code_grant();
        let state = client.get_state().unwrap().clone();
        client.callback_auth_code_grant("test_code", &state).await.unwrap();

        // Wait for token to expire
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        // This should trigger a refresh
        let result = client.get_bearer().await;
        
        assert!(result.is_ok());
        let bearer = result.unwrap();
        assert!(bearer.is_some());
        
        let bearer_string: String = bearer.unwrap().into();
        assert!(bearer_string.contains("new_access_token"));
    }
}