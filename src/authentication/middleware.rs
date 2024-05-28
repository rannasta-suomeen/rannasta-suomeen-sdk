use std::convert::Infallible;

use potion::HtmlError;
use warp::Filter;


pub type Session = Result<SessionData, potion::Error>;

use crate::authentication::jwt::{verify_jwt_session, SessionData};

pub fn with_session() -> impl Filter<Extract = (Session,), Error = Infallible> + Copy {
    warp::cookie::optional::<String>("session").then(|session: Option<String>| async move {
        if session.is_none() {
            return Err(potion::Error::from(
                HtmlError::InvalidSession.new("Invalid session"),
            ));
        }

        if let Ok(data) = verify_jwt_session(session.unwrap()) {
            return Ok::<SessionData, potion::Error>(data.into());
        } else {
            return Err(potion::Error::from(
                HtmlError::InvalidSession.new("Missing session"),
            ));
        }
    })
}

pub fn with_possible_session(
) -> impl Filter<Extract = (Option<SessionData>,), Error = Infallible> + Copy {
    warp::cookie::optional::<String>("session").map(move |session: Option<String>| {
        if session.is_none() {
            return None;
        }

        if let Ok(data) = verify_jwt_session(session.unwrap()) {
            Some(data.into())
        } else {
            None
        }

    })
}
