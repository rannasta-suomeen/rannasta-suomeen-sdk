use hmac::{Hmac, Mac};
use jwt::SignWithKey;
use jwt::VerifyWithKey;
use potion::HtmlError;
use serde::Deserialize;
use serde::Serialize;
use sha2::Sha256;

use crate::database::schema::User;
use crate::schema::UserRole;

use super::permissions::ActionType;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JwtSessionData {
    pub user_id: i32,
    pub username: String,
    pub user_uid: UserRole,
}

impl JwtSessionData {
    pub fn new(id: i32, username: String, uid: UserRole) -> Self {
        Self {
            user_id: id,
            username,
            user_uid: uid,
        }
    }
}

pub type Session = Result<SessionData, potion::Error>;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SessionData {
    pub user_id: i32,
    pub username: String,
    pub user_uid: UserRole,
    pub is_creator: bool,
    pub is_admin: bool,
}

impl SessionData {
    pub fn authenticate(&self, action: ActionType) -> Result<(), potion::Error> {
        if !action.authenticate(&self) {
            return Err(
                HtmlError::Unauthorized.new("You don't have permission to perform this action")
            );
        }
        Ok(())
    }
}

impl Into<SessionData> for JwtSessionData {
    fn into(self) -> SessionData {
        SessionData {
            username: self.username,
            user_id: self.user_id,
            is_creator: self.user_uid == UserRole::Creator,
            is_admin: self.user_uid == UserRole::Admin,
            user_uid: self.user_uid,
        }
    }
}

pub fn generate_jwt_session(user: &User) -> String {
    let key: Hmac<Sha256> = Hmac::new_from_slice(b"secret").unwrap();
    let claims = JwtSessionData::new(user.id, user.username.to_owned(), user.uid.to_owned());

    claims.sign_with_key(&key).unwrap()
}

pub fn verify_jwt_session(token: String) -> Result<JwtSessionData, jwt::Error> {
    let key: Hmac<Sha256> = Hmac::new_from_slice(b"secret").unwrap();

    token.verify_with_key(&key)
}
