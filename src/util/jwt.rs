use std::error::Error;
use jsonwebtoken::{decode, DecodingKey, get_current_timestamp, Validation};
use oauth2::TokenResponse;
use oauth2::basic::BasicTokenResponse;
use serde::Deserialize;

const EXPIRY_LEEWAY: u64 = 10; // In seconds

#[derive(Deserialize)]
struct Claims {
    exp: u64
}

pub fn expired(token: &BasicTokenResponse) -> Result<bool, Box<dyn Error>> {
    let expiry = get_expiry_time(token.access_token().secret())?;
    Ok(expiry - EXPIRY_LEEWAY < get_current_timestamp())
}

pub fn validate(token: BasicTokenResponse) -> Result<BasicTokenResponse, Box<dyn Error>> {
    if token.refresh_token().is_none() {
        return Err("Missing refresh token from auth server token endpoint".into())
    }
    Ok(token)
}

fn get_expiry_time(token: &String) -> Result<u64, Box<dyn Error>> {
    let mut validation = Validation::default();
    // This is NOT insecure because the JWT was just received from the Auth server:
    validation.insecure_disable_signature_validation();
    validation.validate_aud = false;
    let token = decode::<Claims>(token, &DecodingKey::from_secret(&[]), &validation)?;
    Ok(token.claims.exp)
}
