use std::error::Error;
use jsonwebtoken::{decode, DecodingKey, get_current_timestamp, Validation};
use oauth2::{AccessToken, TokenResponse};
use oauth2::basic::BasicTokenResponse;
use serde::Deserialize;

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

pub struct TokenHolder { // TODO: Better name?
    pub token: BasicTokenResponse, // TODO: Better use accessor, but that leads to "cannot move" complaints
    bearer: Bearer, // Bearer token extracted from the access token
    expiry: u64 // Expiry date in seconds since 1970 extracted from the access token
}

impl TokenHolder {
    pub fn new(token: BasicTokenResponse) -> Result<Self, Box<dyn Error>> {
        let bearer = Bearer::from(token.access_token().secret());
        let expiry = get_expiry_time(token.access_token())?;
        Ok(Self { token, bearer, expiry })
    }

    /*
    pub fn token(&self) -> &BasicTokenResponse {
        &self.token
    }
    */

    pub fn bearer(&self) -> Bearer {
        self.bearer.clone()
    }
}

#[derive(Deserialize)]
struct Claims {
    exp: u64
}

pub fn is_expired(token_holder: &TokenHolder) -> bool {
    token_holder.expiry - EXPIRY_LEEWAY < get_current_timestamp()
}

fn get_expiry_time(token: &AccessToken) -> Result<u64, Box<dyn Error>> {
    let token = token.secret();
    let mut validation = Validation::default();
    // This is NOT insecure because the JWT was just received from the Auth server:
    validation.insecure_disable_signature_validation();
    validation.validate_aud = false;
    let token = decode::<Claims>(token, &DecodingKey::from_secret(&[]), &validation)?;
    Ok(token.claims.exp)
}

pub fn validate(token: BasicTokenResponse) -> Result<BasicTokenResponse, Box<dyn Error>> {
    if token.refresh_token().is_none() {
        return Err("Missing refresh token from auth server token endpoint".into())
    }
    Ok(token)
}

