use hmac::{Hmac, Mac};
use jwt::SignWithKey;
use jwt::VerifyWithKey;
use serde::Deserialize;
use serde::Serialize;
use sha2::Sha256;

use crate::database::schema::User;


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JwtSessionData {
    pub user_id: i32,
    pub username: String,
}

impl JwtSessionData {
    pub fn new(id: i32, username: String) -> Self {
        Self {
            user_id: id,
            username
        }
    }
}

pub fn generate_jwt_session(user: &User) -> String {
    let key: Hmac<Sha256> = Hmac::new_from_slice(b"secret").unwrap();
    let claims = JwtSessionData::new(user.id, user.username.to_owned());
    
    claims.sign_with_key(&key).unwrap()
}

pub fn verify_jwt_session(token: String) -> Result<JwtSessionData, jwt::Error> {
    let key: Hmac<Sha256> = Hmac::new_from_slice(b"secret").unwrap();

    token.verify_with_key(&key)
}