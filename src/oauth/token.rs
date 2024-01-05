use std::time::SystemTime;
use oauth2::TokenResponse;
use oauth2::basic::BasicTokenResponse;
use thiserror::Error;

// Number of seconds before expiry time an access token will be refreshed
const EXPIRY_LEEWAY: u64 = 10;

#[derive(Clone, Debug)]
pub struct Bearer(String);

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

pub struct TokenHolder {
    token: BasicTokenResponse,
    bearer: Bearer, // Bearer token extracted from the access token
    expiry: Option<u64> // Expiry date in seconds since 1970
}

impl TokenHolder {
    pub fn new(token: BasicTokenResponse) -> Self {
        let bearer = Bearer::from(format!("Bearer {}", token.access_token().secret()));
        let expiry = token.expires_in().map(|e| e.as_secs() + get_current_time());
        Self { token, bearer, expiry }
    }

    pub fn bearer(&self) -> &Bearer {
        &self.bearer
    }
    pub fn token(&self) -> &BasicTokenResponse {
        &self.token
    }
}

pub fn is_expired(token_holder: &TokenHolder) -> bool {
    token_holder.expiry.map_or(false, |e| e - EXPIRY_LEEWAY < get_current_time())
}

fn get_current_time() -> u64 {
    SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs() // Cannot panic
}

#[derive(Error, Debug)]
pub enum TokenError {
    #[error("Token returned from auth server does not contain a refresh token")]
    RefreshTokenMissing
}

pub fn validate(token: BasicTokenResponse) -> Result<BasicTokenResponse, TokenError> {
    if token.refresh_token().is_none() {
        return Err(TokenError::RefreshTokenMissing)
    }
    Ok(token)
}

