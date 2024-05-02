use std::convert::Infallible;

use warp::{reject::{self, Rejection}, Filter};

use super::jwt::{verify_jwt_session, JwtSessionData};



#[derive(Debug)]
struct Unauthorized;

impl reject::Reject for Unauthorized {}

pub fn with_auth() -> impl Filter<Extract = ((),), Error = Rejection> + Copy {
    warp::cookie::<String>("session").and_then(|session: String| async move {
        if let Ok(_) = verify_jwt_session(session) {
            Ok(())
        } else {
            Err(warp::reject::custom(Unauthorized))
        }
    })
}

pub fn with_session() -> impl Filter<Extract = (JwtSessionData,), Error = Rejection> + Copy {
    warp::cookie::<String>("session").and_then(|session: String| async move {
        if let Ok(data) = verify_jwt_session(session) {
            Ok(data)
        } else {
            Err(warp::reject::custom(Unauthorized))
        }
    })
}

pub fn with_possible_session() -> impl Filter<Extract = (Option<JwtSessionData>,), Error = Rejection> + Copy  {
    warp::cookie::<String>("session").map(move |session: String| {
        if let Ok(data) = verify_jwt_session(session) {
            Some(data)
        } else {
            None
        }
    })
}

