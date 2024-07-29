use potion::HtmlError;
use sqlx::{Pool, Postgres};

use crate::{
    authentication::permissions::ActionType,
    cryptography::generate_access_token,
    error::QueryError,
    schema::{Cabinet, CabinetMember, CabinetProduct},
};

use crate::jwt::SessionData;

use super::{get_product, get_user_by_id};

pub async fn create_cabinet(
    name: &str,
    user_id: i32,
    pool: &Pool<Postgres>,
) -> Result<i32, potion::Error> {
    let id: (i32,) =
        sqlx::query_as("INSERT INTO cabinets (owner_id, name) VALUES ($1, $2) RETURNING *")
            .bind(user_id)
            .bind(name)
            .fetch_one(pool)
            .await
            .map_err(|e| QueryError::from(e).into())?;

    Ok(id.0)
}

pub async fn list_own_cabinets(
    user_id: i32,
    pool: &Pool<Postgres>,
) -> Result<Vec<Cabinet>, potion::Error> {
    let list: Vec<Cabinet> = sqlx::query_as("SELECT * FROM cabinets WHERE owner_id = $1")
        .bind(user_id)
        .fetch_all(pool)
        .await
        .map_err(|e| QueryError::from(e).into())?;

    Ok(list)
}

pub async fn list_friend_cabinets(
    user_id: i32,
    pool: &Pool<Postgres>,
) -> Result<Vec<Cabinet>, potion::Error> {
    let list: Vec<Cabinet> = sqlx::query_as("
        SELECT c.*
        FROM shared_cabinets sc
        INNER JOIN cabinets c ON sc.cabinet_id = c.id
        WHERE sc.user_id = $1
        ")
        .bind(user_id)
        .fetch_all(pool)
        .await
        .map_err(|e| QueryError::from(e).into())?;

    Ok(list)
}

/// Deletes a cabinet with a given id.
/// ATTENTION: DOES NOT CHECK FOR OWNERWHIP BY ITSELF
pub async fn delete_cabinet(id: i32, pool: &Pool<Postgres>) -> Result<(), potion::Error> {
    let mut tr = pool
        .begin()
        .await
        .map_err(|_| QueryError::new("Could not start transaction".to_owned()).into())?;
    sqlx::query("DELETE FROM shared_cabinets WHERE cabinet_id = $1")
        .bind(id)
        .execute(&mut *tr)
        .await
        .map_err(|e| QueryError::from(e).into())?;

    sqlx::query("DELETE FROM cabinet_products WHERE cabinet_id = $1")
        .bind(id)
        .execute(&mut *tr)
        .await
        .map_err(|e| QueryError::from(e).into())?;

    sqlx::query("DELETE FROM cabinets WHERE id = $1")
        .bind(id)
        .execute(&mut *tr)
        .await
        .map_err(|e| QueryError::from(e).into())?;
    tr.commit()
        .await
        .map_err(|_| QueryError::new("Could not commit transaction".to_owned()).into())?;
    Ok(())
}

pub async fn get_cabinet(id: i32, pool: &Pool<Postgres>) -> Result<Option<Cabinet>, potion::Error> {
    let cabinet: Option<Cabinet> = sqlx::query_as("SELECT * FROM cabinets WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(|e| QueryError::from(e).into())?;

    Ok(cabinet)
}

pub async fn get_cabinet_by_token(
    token: &str,
    pool: &Pool<Postgres>,
) -> Result<Option<Cabinet>, potion::Error> {
    let cabinet: Option<Cabinet> = sqlx::query_as("SELECT * FROM cabinets WHERE access_key = $1")
        .bind(token)
        .fetch_optional(pool)
        .await
        .map_err(|e| QueryError::from(e).into())?;

    Ok(cabinet)
}

pub async fn get_cabinet_mut(
    id: i32,
    session: SessionData,
    pool: &Pool<Postgres>,
) -> Result<Cabinet, potion::Error> {
    let cabinet = get_cabinet(id, pool).await?;

    session.authenticate(ActionType::ManageOwnCabinets)?;

    match cabinet {
        Some(cabinet) => match session.authenticate(ActionType::ManageAllCabinets) {
            Ok(_) => Ok(cabinet),
            Err(_) => {
                if cabinet.owner_id != session.user_id {
                    if list_cabinet_access_list(id, &pool)
                        .await?
                        .iter()
                        .any(|member| member.user_id == session.user_id)
                    {
                        Ok(cabinet)
                    } else {
                        Err(HtmlError::Unauthorized.default())
                    }
                } else {
                    Ok(cabinet)
                }
            }
        },
        None => Err(HtmlError::InvalidRequest.new("No recipe exists with spcified id")),
    }
}

pub async fn list_cabinet_products(
    id: i32,
    pool: &Pool<Postgres>,
) -> Result<Vec<CabinetProduct>, potion::Error> {
    let list: Vec<CabinetProduct> =
        sqlx::query_as("SELECT * FROM cabinet_products WHERE cabinet_id = $1")
            .bind(id)
            .fetch_all(pool)
            .await
            .map_err(|e| QueryError::from(e).into())?;

    Ok(list)
}

pub async fn list_cabinet_access_list(
    id: i32,
    pool: &Pool<Postgres>,
) -> Result<Vec<CabinetMember>, potion::Error> {
    let list: Vec<CabinetMember> =
        sqlx::query_as("SELECT * FROM shared_cabinets WHERE cabinet_id = $1")
            .bind(id)
            .fetch_all(pool)
            .await
            .map_err(|e| QueryError::from(e).into())?;

    Ok(list)
}

pub async fn modify_in_cabinet(
    id: i32,
    product_id: i32,
    amount_ml: Option<i32>,
    pool: &Pool<Postgres>,
) -> Result<(), potion::Error> {
    sqlx::query(
        "UPDATE cabinet_products SET amount_ml = $1 WHERE cabinet_id = $2 AND id = $3",
    )
    .bind(amount_ml)
    .bind(id)
    .bind(product_id)
    .execute(pool)
    .await
    .map_err(|e| QueryError::from(e).into())?;
    Ok(())
}

pub async fn add_to_cabinet(
    id: i32,
    user_id: i32,
    product_id: i32,
    amount_ml: Option<i32>,
    pool: &Pool<Postgres>,
) -> Result<(), potion::Error> {
    let product = get_product(product_id, pool).await?;
    if product.is_none() {
        return Err(HtmlError::InvalidRequest.new("Product with specified id doesn't exists"));
    }
    let product = product.unwrap();

    let result = sqlx::query(
        "INSERT INTO cabinet_products (cabinet_id, product_id, owner_id, name, img, href, abv, amount_ml)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            ON CONFLICT DO NOTHING RETURNING *",
    )
    .bind(id)
    .bind(product.id)
    .bind(user_id)
    .bind(product.name)
    .bind(product.img)
    .bind(product.href)
    .bind(product.abv)
    .bind(amount_ml)
    .execute(pool)
    .await
    .map_err(|e| QueryError::from(e).into())?;

    if result.rows_affected() <= 0 {
        return Err(HtmlError::InvalidRequest
            .new("Product is already in the cabinet")
            .into());
    }

    Ok(())
}

pub async fn remove_from_cabinet(
    id: i32,
    product_id: i32,
    pool: &Pool<Postgres>,
) -> Result<(), potion::Error> {
    let result =
        sqlx::query("DELETE FROM cabinet_products WHERE cabinet_id = $1 AND id = $2")
            .bind(id)
            .bind(product_id)
            .execute(pool)
            .await
            .map_err(|e| QueryError::from(e).into())?;

    if result.rows_affected() <= 0 {
        return Err(HtmlError::InvalidRequest
            .new("Product was already removed from the cabinet")
            .into());
    }

    Ok(())
}

pub async fn set_product_unusable(
    id: i32,
    product_id: i32,
    pool: &Pool<Postgres>,
) -> Result<(), potion::Error> {
    sqlx::query(
        "UPDATE cabinet_products SET usable = false WHERE cabinet_id = $1 AND id = $2",
    )
    .bind(id)
    .bind(product_id)
    .execute(pool)
    .await
    .map_err(|e| QueryError::from(e).into())?;

    Ok(())
}

pub async fn set_product_usable(
    id: i32,
    product_id: i32,
    pool: &Pool<Postgres>,
) -> Result<(), potion::Error> {
    sqlx::query(
        "UPDATE cabinet_products SET usable = true WHERE cabinet_id = $1 AND id = $2",
    )
    .bind(id)
    .bind(product_id)
    .execute(pool)
    .await
    .map_err(|e| QueryError::from(e).into())?;

    Ok(())
}

pub async fn set_cabinet_name(
    id: i32,
    name: &str,
    pool: &Pool<Postgres>,
) -> Result<(), potion::Error> {
    sqlx::query("UPDATE cabinets SET name = $1 WHERE id = $2")
        .bind(name)
        .bind(id)
        .execute(pool)
        .await
        .map_err(|e| QueryError::from(e).into())?;

    Ok(())
}

pub async fn set_product_amount(
    id: i32,
    product_id: i32,
    amount: Option<i32>,
    pool: &Pool<Postgres>,
) -> Result<(), potion::Error> {
    sqlx::query(
        "UPDATE cabinet_products SET amount_ml = $1 WHERE cabinet_id = $2 AND id = $3",
    )
    .bind(amount)
    .bind(id)
    .bind(product_id)
    .execute(pool)
    .await
    .map_err(|e| QueryError::from(e).into())?;

    Ok(())
}

pub async fn generate_cabinet_access_token(
    id: i32,
    pool: &Pool<Postgres>,
) -> Result<(), potion::Error> {
    let token = generate_access_token();

    sqlx::query("UPDATE cabinets SET access_key = $1 WHERE id = $2")
        .bind(token)
        .bind(id)
        .execute(pool)
        .await
        .map_err(|e| QueryError::from(e).into())?;

    Ok(())
}

pub async fn add_user_to_cabinet(
    id: i32,
    user_id: i32,
    pool: &Pool<Postgres>,
) -> Result<(), potion::Error> {
    let user = get_user_by_id(pool, user_id).await?;
    if user.is_none() {
        return Err(HtmlError::InvalidRequest.new("User doesn't exists"));
    }

    sqlx::query(
        "INSERT INTO shared_cabinets (cabinet_id, user_id, user_username) VALUES ($1, $2, $3)",
    )
    .bind(id)
    .bind(user_id)
    .bind(user.unwrap().username)
    .execute(pool)
    .await
    .map_err(|e| QueryError::from(e).into())?;

    Ok(())
}

pub async fn remove_user_from_cabinet(
    id: i32,
    user_id: i32,
    pool: &Pool<Postgres>,
) -> Result<(), potion::Error> {

    sqlx::query(
        "DELETE FROM shared_cabinets WHERE cabinet_id = $1 AND user_id = $2",
    )
    .bind(id)
    .bind(user_id)
    .execute(pool)
    .await
    .map_err(|e| QueryError::from(e).into())?;

    sqlx::query(
        "DELETE FROM cabinet_products WHERE cabinet_id = $1 AND owner_id = $2",
    )
    .bind(id)
    .bind(user_id)
    .execute(pool)
    .await
    .map_err(|e| QueryError::from(e).into())?;

    Ok(())
}

