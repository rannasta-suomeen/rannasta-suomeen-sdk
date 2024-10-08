use potion::HtmlError;
use sqlx::{FromRow, Pool, Postgres, QueryBuilder};

use crate::{
    authentication::permissions::ActionType,
    cryptography::generate_access_token,
    error::QueryError,
    schema::{Cabinet, CabinetMember, CabinetMixer, CabinetMixerOwned, CabinetProduct},
};

use crate::jwt::SessionData;

use super::{get_incredient, get_product, get_user_by_id};

pub async fn update_cabinet_checksum(id: i32, pool: &Pool<Postgres>) -> Result<(), potion::Error> {
    let key = uuid::Uuid::new_v4().to_string();

    sqlx::query("UPDATE cabinets SET checksum = $1 WHERE id = $2")
        .bind(key)
        .bind(id)
        .execute(pool)
        .await
        .map_err(|e| QueryError::from(e).into())?;

    Ok(())
}

pub async fn create_cabinet(
    name: &str,
    user_id: i32,
    pool: &Pool<Postgres>,
) -> Result<i32, potion::Error> {
    let key = uuid::Uuid::new_v4().to_string();

    let id: (i32,) = sqlx::query_as(
        "INSERT INTO cabinets (owner_id, name, checksum) VALUES ($1, $2, $3) RETURNING *",
    )
    .bind(user_id)
    .bind(name)
    .bind(key)
    .fetch_one(pool)
    .await
    .map_err(|e| QueryError::from(e).into())?;

    add_user_to_cabinet(id.0, user_id, &pool).await?;

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

/// Also lists owned cabinets, just includes friend cabinets too
pub async fn list_friend_cabinets(
    user_id: i32,
    pool: &Pool<Postgres>,
) -> Result<Vec<Cabinet>, potion::Error> {
    let list: Vec<Cabinet> = sqlx::query_as(
        "
        SELECT c.*
        FROM shared_cabinets sc
        INNER JOIN cabinets c ON sc.cabinet_id = c.id
        WHERE sc.user_id = $1
        ",
    )
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

    sqlx::query("DELETE FROM cabinet_mixers WHERE cabinet_id = $1")
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
        sqlx::query_as("SELECT * FROM cabinet_products WHERE cabinet_id = $1 ORDER BY id")
            .bind(id)
            .fetch_all(pool)
            .await
            .map_err(|e| QueryError::from(e).into())?;

    Ok(list)
}

pub async fn list_cabinet_mixers_rsm(
    id: i32,
    pool: &Pool<Postgres>,
) -> Result<Vec<CabinetMixer>, potion::Error> {
    let list: Vec<CabinetMixer> =
        sqlx::query_as("SELECT * FROM cabinet_mixers WHERE cabinet_id = $1 ORDER BY id")
            .bind(id)
            .fetch_all(pool)
            .await
            .map_err(|e| QueryError::from(e).into())?;
    Ok(list)
}

pub async fn list_cabinet_mixers(
    id: i32,
    pool: &Pool<Postgres>,
) -> Result<Vec<CabinetMixerOwned>, potion::Error> {
    let list: Vec<CabinetMixer> =
        sqlx::query_as("SELECT * FROM cabinet_mixers WHERE cabinet_id = $1 ORDER BY id")
            .bind(id)
            .fetch_all(pool)
            .await
            .map_err(|e| QueryError::from(e).into())?;

    let b: Vec<CabinetMixerOwned> = vec![];
    let list = list.into_iter().fold(b, |mut a, v| {
        if let Some(c) = a.iter_mut().find(|m| m.incredient_id == v.incredient_id) {
            c.owners.push(v.owner_id);
            if let Some(amount) = v.amount {
                c.owner_map.insert(v.owner_id, v.amount);
                match c.amount {
                    Some(c_amount) => c.amount = Some(c_amount + amount),
                    None => c.amount = Some(amount),
                }
            } else {
                c.amount = None;
            }
        } else {
            a.push(CabinetMixerOwned::from(v));
        }
        a
    });

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

pub async fn get_cabinet_mixer(
    i_id: i32,
    pool: &Pool<Postgres>,
) -> Result<Option<CabinetMixer>, potion::Error> {
    let mixer: Option<CabinetMixer> = sqlx::query_as("SELECT * FROM cabinet_mixers WHERE id = $1")
        .bind(i_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| QueryError::from(e).into())?;

    Ok(mixer)
}

pub async fn get_cabinet_mixer_owned(
    id: i32,
    incredient_id: i32,
    user_id: i32,
    pool: &Pool<Postgres>,
) -> Result<Option<CabinetMixer>, potion::Error> {
    let mixer: Option<CabinetMixer> = sqlx::query_as("SELECT * FROM cabinet_mixers WHERE cabinet_id = $1 AND incredient_id = $2 AND owner_id = $3")
        .bind(id)
        .bind(incredient_id)
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| QueryError::from(e).into())?;

    Ok(mixer)
}

pub async fn modify_mixer_in_cabinet(
    id: i32,
    incredient_id: i32,
    user_id: i32,
    amount: Option<i32>,
    pool: &Pool<Postgres>,
) -> Result<(), potion::Error> {
    let mixer = get_cabinet_mixer_owned(id, incredient_id, user_id, pool).await?;
    if mixer.is_none() {
        add_mixer_to_cabinet(id, user_id, incredient_id, amount, pool).await?;
    } else {
        sqlx::query("UPDATE cabinet_mixers SET amount = $1 WHERE cabinet_id = $2 AND incredient_id = $3 AND owner_id = $4")
            .bind(amount)
            .bind(id)
            .bind(incredient_id)
            .bind(user_id)
            .execute(pool)
            .await
            .map_err(|e| QueryError::from(e).into())?;
    }

    update_cabinet_checksum(id, pool).await?;

    Ok(())
}

pub async fn modify_mixer_in_cabinet_rsm(
    id: i32,
    mixer_id: i32,
    amount: Option<i32>,
    pool: &Pool<Postgres>,
) -> Result<(), potion::Error> {
    sqlx::query("UPDATE cabinet_mixers SET amount = $1 WHERE id = $2")
        .bind(amount)
        .bind(mixer_id)
        .execute(pool)
        .await
        .map_err(|e| QueryError::from(e).into())?;

    update_cabinet_checksum(id, pool).await?;

    Ok(())
}

/// DOES NOT CHECK OWNERSHIP OR RIGHTS
pub async fn set_mixer_usable(
    id: i32,
    cabinet_id: i32,
    pool: &Pool<Postgres>,
) -> Result<(), potion::Error> {
    sqlx::query("UPDATE cabinet_mixers SET usable = true WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await
        .map_err(|e| QueryError::from(e).into())?;

    update_cabinet_checksum(cabinet_id, pool).await?;
    Ok(())
}

/// DOES NOT CHECK OWNERSHIP OR RIGHTS
pub async fn set_mixer_unusable(
    id: i32,
    cabinet_id: i32,
    pool: &Pool<Postgres>,
) -> Result<(), potion::Error> {
    sqlx::query("UPDATE cabinet_mixers SET usable = false WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await
        .map_err(|e| QueryError::from(e).into())?;

    update_cabinet_checksum(cabinet_id, pool).await?;
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

    update_cabinet_checksum(id, pool).await?;

    Ok(())
}

pub async fn add_mixer_to_cabinet(
    id: i32,
    user_id: i32,
    ingredient_id: i32,
    amount_ml: Option<i32>,
    pool: &Pool<Postgres>,
) -> Result<(), potion::Error> {
    let ingredient = get_incredient(ingredient_id, pool).await?;
    if ingredient.is_none() {
        return Err(HtmlError::InvalidRequest.new("Product with specified id doesn't exists"));
    }
    let incredient = ingredient.unwrap();

    let result = sqlx::query(
        "INSERT INTO cabinet_mixers (cabinet_id, incredient_id, owner_id, name, unit, amount)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT DO NOTHING RETURNING *",
    )
    .bind(id)
    .bind(incredient.id)
    .bind(user_id)
    .bind(incredient.name)
    .bind(incredient.unit)
    .bind(amount_ml)
    .execute(pool)
    .await
    .map_err(|e| QueryError::from(e).into())?;

    if result.rows_affected() <= 0 {
        return Err(HtmlError::InvalidRequest
            .new("Mixer is already in the cabinet")
            .into());
    }

    update_cabinet_checksum(id, pool).await?;

    Ok(())
}

/// Note: This method will perform automatic checks to determine the ownerships of imported products
/// * This is due to the fact that such check would be impossible to implement outside this method withot additiona overhead
pub async fn add_to_cabinet_bulk(
    cabinet_id: i32,
    id_map: &[i32],
    user_id: i32,
    pool: &Pool<Postgres>,
) -> Result<(), potion::Error> {
    let product_list: Vec<CabinetProduct> = fetch_cabinet_products(&id_map, &pool)
        .await?
        .drain(..)
        .filter(|p| p.owner_id == user_id)
        .collect();

    insert_cabinet_products(cabinet_id, &product_list, user_id, &pool).await?;

    Ok(())
}

pub async fn fetch_cabinet_products(
    id_map: &[i32],
    pool: &Pool<Postgres>,
) -> Result<Vec<CabinetProduct>, potion::Error> {
    if id_map.len() <= 0 {
        return Ok(vec![]);
    }

    let mut query_builder: QueryBuilder<Postgres> =
        QueryBuilder::new("SELECT * FROM cabinet_products WHERE id IN ");

    query_builder.push_tuples(id_map.iter().take(65535 / 4), |mut b, id| {
        b.push_bind(id);
    });

    let list: Vec<CabinetProduct> = query_builder
        .build()
        .fetch_all(&*pool)
        .await
        .map_err(|e| QueryError::from(e).into())?
        .iter()
        .filter_map(|row| CabinetProduct::from_row(row).ok())
        .collect();

    Ok(list)
}

pub async fn insert_cabinet_products(
    cabinet_id: i32,
    product_map: &[CabinetProduct],
    user_id: i32,
    pool: &Pool<Postgres>,
) -> Result<(), potion::Error> {
    if product_map.len() <= 0 {
        return Ok(());
    }

    let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
        "INSERT INTO cabinet_products (cabinet_id, product_id, owner_id, name, img, href, abv, amount_ml) ",
    );

    query_builder.push_values(product_map.iter().take(65535 / 4), |mut b, product| {
        b.push_bind(cabinet_id)
            .push_bind(&product.product_id)
            .push_bind(user_id)
            .push_bind(&product.name)
            .push_bind(&product.img)
            .push_bind(&product.href)
            .push_bind(&product.abv)
            .push_bind(&product.amount_ml);
    });

    query_builder
        .build()
        .execute(&*pool)
        .await
        .map_err(|e| QueryError::from(e).into())?;

    update_cabinet_checksum(cabinet_id, pool).await?;

    Ok(())
}

pub async fn remove_from_cabinet(
    id: i32,
    product_id: i32,
    pool: &Pool<Postgres>,
) -> Result<(), potion::Error> {
    let result = sqlx::query("DELETE FROM cabinet_products WHERE id = $1")
        .bind(product_id)
        .execute(pool)
        .await
        .map_err(|e| QueryError::from(e).into())?;

    if result.rows_affected() <= 0 {
        return Err(HtmlError::InvalidRequest
            .new("Product was already removed from the cabinet")
            .into());
    }

    update_cabinet_checksum(id, pool).await?;

    Ok(())
}

/// Removes a mixer that you own from a cabinet
// TODO: Move checking for ownership somewhere else so that admins are able to modify products not
// owned by themselves
pub async fn remove_mixer_from_cabinet(
    id: i32,
    incredient_id: i32,
    user_id: i32,
    pool: &Pool<Postgres>,
) -> Result<(), potion::Error> {
    let mixer = get_cabinet_mixer_owned(id, incredient_id, user_id, pool).await?;
    if mixer.is_none() {
        return Err(HtmlError::InvalidRequest.new("Mixer doesn't exists"));
    }
    let mixer = mixer.unwrap();

    let result =
        sqlx::query("DELETE FROM cabinet_mixers WHERE cabinet_id = $1 AND incredient_id = $2")
            .bind(id)
            .bind(mixer.incredient_id)
            .execute(pool)
            .await
            .map_err(|e| QueryError::from(e).into())?;

    if result.rows_affected() <= 0 {
        return Err(HtmlError::InvalidRequest
            .new("Product was already removed from the cabinet")
            .into());
    }

    update_cabinet_checksum(id, pool).await?;

    Ok(())
}

/// DOES NOT CHECK FOR OWNERSHIP
pub async fn remove_mixer_from_cabinet_rsm(
    id: i32,
    mixer_id: i32,
    pool: &Pool<Postgres>,
) -> Result<(), potion::Error> {

    let result =
        sqlx::query("DELETE FROM cabinet_mixers WHERE id = $1")
            .bind(mixer_id)
            .execute(pool)
            .await
            .map_err(|e| QueryError::from(e).into())?;

    if result.rows_affected() <= 0 {
        return Err(HtmlError::InvalidRequest
            .new("Product was already removed from the cabinet")
            .into());
    }

    update_cabinet_checksum(id, pool).await?;

    Ok(())
}

pub async fn set_product_unusable(
    id: i32,
    product_id: i32,
    pool: &Pool<Postgres>,
) -> Result<(), potion::Error> {
    sqlx::query("UPDATE cabinet_products SET usable = false WHERE id = $1")
        .bind(product_id)
        .execute(pool)
        .await
        .map_err(|e| QueryError::from(e).into())?;

    update_cabinet_checksum(id, pool).await?;

    Ok(())
}

pub async fn set_product_usable(
    id: i32,
    product_id: i32,
    pool: &Pool<Postgres>,
) -> Result<(), potion::Error> {
    sqlx::query("UPDATE cabinet_products SET usable = true WHERE id = $1")
        .bind(product_id)
        .execute(pool)
        .await
        .map_err(|e| QueryError::from(e).into())?;

    update_cabinet_checksum(id, pool).await?;

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

    update_cabinet_checksum(id, pool).await?;

    Ok(())
}

pub async fn set_product_amount(
    id: i32,
    product_id: i32,
    amount: Option<i32>,
    pool: &Pool<Postgres>,
) -> Result<(), potion::Error> {
    sqlx::query("UPDATE cabinet_products SET amount_ml = $1 WHERE id = $2")
        .bind(amount)
        .bind(product_id)
        .execute(pool)
        .await
        .map_err(|e| QueryError::from(e).into())?;

    update_cabinet_checksum(id, pool).await?;

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

    update_cabinet_checksum(id, pool).await?;

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
    sqlx::query("DELETE FROM shared_cabinets WHERE cabinet_id = $1 AND user_id = $2")
        .bind(id)
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(|e| QueryError::from(e).into())?;

    sqlx::query("DELETE FROM cabinet_products WHERE cabinet_id = $1 AND owner_id = $2")
        .bind(id)
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(|e| QueryError::from(e).into())?;

    sqlx::query("DELETE FROM cabinet_mixers WHERE cabinet_id = $1 AND owner_id = $2")
        .bind(id)
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(|e| QueryError::from(e).into())?;

    Ok(())
}
