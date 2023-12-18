use std::error::Error;
use jsonwebtoken::{decode, DecodingKey, get_current_timestamp, Validation};
use oauth2::{AccessToken, TokenResponse};
use serde::Deserialize;
use log::warn;
use oauth2::basic::BasicTokenResponse;

const EXPIRY_LEEWAY: u64 = 10; // In seconds

#[derive(thiserror::Error,Debug)]
enum TokenError {
    #[error("Missing refresh token")]
    RefreshTokenMissing
}

#[derive(Deserialize)]
struct Claims {
    exp: u64
}

pub fn expired(expiry: &u64) -> bool {
    return expiry - EXPIRY_LEEWAY < get_current_timestamp();
}

pub fn validate(token: BasicTokenResponse) -> Result<(BasicTokenResponse, u64), Box<dyn Error>> {
    if token.refresh_token().is_none() {
        warn!("Refresh token missing in result from auth server token endpoint");
        return Err(Box::new(TokenError::RefreshTokenMissing));
    }
    let expiry = get_expiry_time(token.access_token())?;
    Ok((token, expiry))
}

fn get_expiry_time(token: &AccessToken) -> Result<u64, Box<dyn Error>> {
    let token = token.secret();
    //info!("--t--> {}", token);
    let mut validation = Validation::default();
    // This is NOT insecure because the JWT was just received from the Auth server:
    validation.insecure_disable_signature_validation();
    validation.validate_aud = false;
    let token = decode::<Claims>(token, &DecodingKey::from_secret(&[]), &validation)?;
    //info!("--t--> {:?}", token.header);
    //info!("--t--> {}", token.claims.exp);
    Ok(token.claims.exp)
}
