use crate::{
    authentication::{cryptography::verify_password, jwt::generate_jwt_session},
    error::QueryError,
    schema::User,
};

use potion::HtmlError;
use sqlx::{Pool, Postgres};

pub async fn get_user(
    pool: &Pool<Postgres>,
    username: &str,
) -> Result<Option<User>, potion::Error> {
    let row: Option<User> = sqlx::query_as("SELECT * FROM users WHERE username = $1")
        .bind(username)
        .fetch_optional(&*pool)
        .await
        .map_err(|e| QueryError::from(e).into())?;

    Ok(row)
}

pub async fn get_user_by_id(
    pool: &Pool<Postgres>,
    user_id: i32,
) -> Result<Option<User>, potion::Error> {
    let row: Option<User> = sqlx::query_as("SELECT * FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_optional(&*pool)
        .await
        .map_err(|e| QueryError::from(e).into())?;

    Ok(row)
}

/// Creates a user with username and password, which is the hashed version of their password
pub async fn register_user(
    username: &str,
    password: &str,
    pool: &Pool<Postgres>,
) -> Result<bool, potion::Error> {
    let query = sqlx::query(
        "
        INSERT INTO users (username, password)
        VALUES ($1, $2)
        ON CONFLICT DO NOTHING RETURNING *;
    ",
    )
    .bind(username)
    .bind(password)
    .execute(&*pool)
    .await
    .map_err(|e| QueryError::from(e).into())?;

    Ok(query.rows_affected() > 0)
}

pub async fn login_user(
    username: &str,
    password: &str,
    pool: &Pool<Postgres>,
) -> Result<String, potion::Error> {
    let user = get_user(pool, username).await?;
    if user.is_none() {
        return Err(HtmlError::InvalidRequest.new("Invalid credentials"));
    }

    let user = user.unwrap();
    let authenticated = verify_password(password, &user.password)
        .map_err(|_e| panic!("!"))
        .unwrap();
    if !authenticated {
        return Err(HtmlError::InvalidRequest.new("Invalid credentials"));
    }

    let session = generate_jwt_session(&user);

    Ok(session)
}
