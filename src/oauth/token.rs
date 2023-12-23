use std::error::Error;
use std::time::SystemTime;
use oauth2::TokenResponse;
use oauth2::basic::BasicTokenResponse;

const EXPIRY_LEEWAY: u64 = 10; // In seconds

#[derive(Clone, Debug)]
pub struct Bearer(String);

impl From<&String> for Bearer {
    fn from(item: &String) -> Self {
        Self { 0: item.clone() }
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
    expiry: Option<u64> // Expiry date in seconds since 1970 extracted from the access token
}

impl TokenHolder {
    pub fn new(token: BasicTokenResponse) -> Result<Self, Box<dyn Error>> {
        let bearer = Bearer::from(token.access_token().secret());
        let expiry = token.expires_in().map(|e| e.as_secs() + get_current_time());
        Ok(Self { token, bearer, expiry })
    }

    pub fn bearer(&self) -> Bearer {
        self.bearer.clone()
    }
    pub fn token(&self) -> &BasicTokenResponse {
        &self.token
    }
}

pub fn is_expired(token_holder: &TokenHolder) -> bool {
    token_holder.expiry.map_or(false, |e| e  - EXPIRY_LEEWAY < get_current_time())
}

fn get_current_time() -> u64 {
    SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs() // Cannot panic
}

pub fn validate(token: BasicTokenResponse) -> Result<BasicTokenResponse, Box<dyn Error>> {
    if token.refresh_token().is_none() {
        return Err("Missing refresh token from auth server token endpoint".into())
    }
    Ok(token)
}

