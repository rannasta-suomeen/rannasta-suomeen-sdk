use std::collections::HashMap;

use crate::{
    authentication::{
        cryptography::verify_password, jwt::generate_jwt_session, permissions::ActionType,
    },
    cryptography::generate_access_token,
    schema::{Cabinet, CabinetMember, CabinetProduct, LinkedRecipeTag, RecipeRow, RecipeTag},
};

use super::{
    error::QueryError,
    schema::{
        Category, Incredient, IncredientFilterObject, ProductType, Recipe, RecipePart, RecipeType,
        SubCategory, UnitType, User,
    },
};
use crate::{
    constants::PRODUCT_COUNT_PER_PAGE,
    jwt::SessionData,
    schema::{
        IncredientCacheData, IncredientFilterObjectNoName, IncredientOrder, IncredientRow,
        IngredientFilterList, IngredientsForDrink, Product, ProductRow, RecipeAvailability,
        RecipeCacheData, RecipeOrder, RecipePartNoId, RecipeRowPartial, Uuid,
    },
    INCREDIENT_COUNT_PER_PAGE, RECIPE_COUNT_PER_PAGE,
};
use potion::{pagination::PageContext, HtmlError};
use sqlx::{Pool, Postgres, QueryBuilder};

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

pub async fn list_recipes(pool: &Pool<Postgres>) -> Result<Vec<Recipe>, potion::Error> {
    let rows: Vec<Recipe> = sqlx::query_as("SELECT * FROM drink_recipes;")
        .fetch_all(&*pool)
        .await
        .map_err(|e| QueryError::from(e).into())?;

    Ok(rows)
}

pub async fn fetch_recipes(
    category: Option<RecipeType>,
    order: Option<RecipeOrder>,
    availability: Option<RecipeAvailability>,
    offset: i64,
    search: String,
    pool: &Pool<Postgres>,
) -> Result<PageContext<RecipeRow>, potion::Error> {
    let availability = availability
        .map(|availability| match availability {
            RecipeAvailability::Any => "",
            RecipeAvailability::Alko => "AND r.available_alko",
            RecipeAvailability::Superalko => "AND r.available_superalko",
        })
        .unwrap_or("");

    let order = order
        .map(|order| match order {
            RecipeOrder::Alphabetical => "name",
            RecipeOrder::AbvAsc => "abv_average",
            RecipeOrder::AbvDesc => "abv_average DESC",
            RecipeOrder::VolumeAsc => "total_volume",
            RecipeOrder::VolumeDesc => "total_volume DESC",
            RecipeOrder::ServingsAsc => "standard_servings",
            RecipeOrder::ServingsDesc => "standard_servings DESC",
            RecipeOrder::IncredientCountAsc => "incredient_count",
            RecipeOrder::IncredientCountDesc => "incredient_count DESC",
            RecipeOrder::PriceSuperalkoAsc => "superalko_price_min",
            RecipeOrder::PriceSuperalkoDesc => "superalko_price_max DESC",
            RecipeOrder::PriceAlkoAsc => "alko_price_min",
            RecipeOrder::PriceAlkoDesc => "alko_price_max DESC",
        })
        .unwrap_or("name");

    let rows: Vec<RecipeRowPartial> = match category {
        Some(category)=> {
            sqlx::query_as(&format!("SELECT r.*, COUNT(rr) OVER() FROM drink_recipes r LEFT JOIN drink_recipes rr ON rr.id = r.id WHERE r.type = $1 AND r.name ILIKE $2 {availability} ORDER BY {order} LIMIT $3 OFFSET $4"))
                .bind(category)
                .bind(search)
                .bind(RECIPE_COUNT_PER_PAGE)
                .bind(offset)
                .fetch_all(&*pool).await.map_err(|e| QueryError::from(e).into())?
        },
        None => {
            sqlx::query_as(&format!("SELECT r.*, COUNT(rr) OVER() FROM drink_recipes r LEFT JOIN drink_recipes rr ON rr.id = r.id WHERE r.name ILIKE $1 {availability} ORDER BY {order} LIMIT $2 OFFSET $3"))
                .bind(search)
                .bind(RECIPE_COUNT_PER_PAGE)
                .bind(offset)
                .fetch_all(&*pool).await.map_err(|e| QueryError::from(e).into())?
        },
    };

    let rows: Vec<RecipeRow> = rows.into_iter().map(|row| RecipeRow::from(row)).collect();

    let total_count = *&rows.get(0).map(|p| p.count).unwrap_or(0);
    let page = PageContext::from_rows(rows, total_count, RECIPE_COUNT_PER_PAGE, offset);
    Ok(page)
}

pub async fn list_recipe_parts(
    pool: &Pool<Postgres>,
    recipe_id: i32,
) -> Result<Vec<RecipePart>, potion::Error> {
    let rows: Vec<RecipePart> = sqlx::query_as("
        SELECT rp.recipe_id AS recipe_id, d.id AS incredient_id, rp.amount AS amount, rp.unit AS unit, d.name AS name
        FROM recipe_parts rp
        INNER JOIN drink_incredients d ON d.id = rp.incredient_id
        WHERE rp.recipe_id = $1
    ")
    .bind(recipe_id)
    .fetch_all(pool).await.map_err(|e| QueryError::from(e).into())?;

    Ok(rows)
}

pub async fn list_all_recipe_parts(
    pool: &Pool<Postgres>,
) -> Result<Vec<IngredientsForDrink>, potion::Error> {
    let filters: Vec<RecipePart> = sqlx::query_as("SELECT r.recipe_id AS recipe_id, r.incredient_id AS incredient_id, r.amount AS amount, r.unit AS unit, d.name AS name
                                                  FROM recipe_parts r
                                                  INNER JOIN drink_incredients d ON d.id = r.incredient_id")
        .fetch_all(pool).await.map_err(|e| QueryError::from(e).into())?;
    let mut hashmap: HashMap<Uuid, Vec<RecipePartNoId>> = HashMap::new();
    filters
        .into_iter()
        .for_each(|x| match hashmap.get_mut(&x.recipe_id) {
            Some(v) => v.push(x.into()),
            None => {
                hashmap.insert(x.recipe_id, vec![x.into()]);
            }
        });
    Ok(hashmap
        .into_iter()
        .map(|(recipe_id, recipe_parts)| IngredientsForDrink {
            recipe_id,
            recipe_parts,
        })
        .collect())
}

pub async fn create_recipe(
    category: RecipeType,
    user_id: i32,
    name: String,
    pool: &Pool<Postgres>,
) -> Result<i32, potion::Error> {
    let recipe: (i32,) = sqlx::query_as("INSERT INTO recipes DEFAULT VALUES RETURNING id")
        .fetch_one(pool)
        .await
        .map_err(|e| QueryError::from(e).into())?;

    let recipe_id = recipe.0;

    let id: (i32,) = sqlx::query_as(
        "
        INSERT INTO drink_recipes (type, author_id, name, recipe_id)
        VALUES ($1, $2, $3, $4)
        RETURNING id
    ",
    )
    .bind(category)
    .bind(user_id)
    .bind(name)
    .bind(recipe_id)
    .fetch_one(pool)
    .await
    .map_err(|e| QueryError::from(e).into())?;

    Ok(id.0)
}

pub async fn find_recipe(name: &str, pool: &Pool<Postgres>) -> Result<Option<i32>, potion::Error> {
    let row: Option<(i32,)> =
        sqlx::query_as("SELECT id FROM drink_recipes WHERE LOWER(name) = LOWER($1)")
            .bind(name)
            .fetch_optional(&*pool)
            .await
            .map_err(|e| QueryError::from(e).into())?;

    Ok(row.map(|r| r.0))
}

pub async fn get_recipe(id: i32, pool: &Pool<Postgres>) -> Result<Option<Recipe>, potion::Error> {
    let row: Option<Recipe> = sqlx::query_as("SELECT * FROM drink_recipes WHERE id = $1")
        .bind(id)
        .fetch_optional(&*pool)
        .await
        .map_err(|e| QueryError::from(e).into())?;

    Ok(row)
}

pub async fn get_recipe_mut(
    id: i32,
    session: SessionData,
    pool: &Pool<Postgres>,
) -> Result<Recipe, potion::Error> {
    let recipe = get_recipe(id, pool).await?;
    session.authenticate(ActionType::ManageOwnRecipes)?;

    match recipe {
        Some(recipe) => match session.authenticate(ActionType::ManageAllRecipes) {
            Ok(_) => Ok(recipe),
            Err(_) => {
                if recipe.author_id != session.user_id {
                    Err(HtmlError::Unauthorized.default())
                } else {
                    Ok(recipe)
                }
            }
        },
        None => Err(HtmlError::InvalidRequest.new("No recipe exists with spcified id")),
    }
}

pub async fn update_recipe_info(
    id: i32,
    name: String,
    category: RecipeType,
    info: String,
    pool: &Pool<Postgres>,
) -> Result<(), potion::Error> {
    sqlx::query("UPDATE drink_recipes SET name = $1, type = $2, info = $3 WHERE id = $4")
        .bind(name)
        .bind(category)
        .bind(info)
        .bind(id)
        .execute(&*pool)
        .await
        .map_err(|e| QueryError::from(e).into())?;

    Ok(())
}

pub async fn add_to_recipe(
    recipe_id: i32,
    base: i32,
    unit: UnitType,
    amount: i32,
    pool: &Pool<Postgres>,
) -> Result<(), potion::Error> {
    let amount_ml = unit.convert(amount.into(), UnitType::Ml).1;

    sqlx::query(
        "
        INSERT INTO recipe_parts (recipe_id, incredient_id, amount, amount_standard, unit) 
        VALUES ($1, $2, $3, $4, $5)
        ON CONFLICT (recipe_id, incredient_id) DO UPDATE
        SET amount = $3, amount_standard = $4, unit = $5;
    ",
    )
    .bind(recipe_id)
    .bind(base)
    .bind(amount)
    .bind(amount_ml)
    .bind(unit)
    .execute(&*pool)
    .await
    .map_err(|e| QueryError::from(e).into())?;

    update_recipe_cached_data(recipe_id, pool).await?;

    Ok(())
}

pub async fn remove_from_recipe(
    recipe_id: i32,
    incredient_id: i32,
    pool: &Pool<Postgres>,
) -> Result<(), potion::Error> {
    sqlx::query(
        "
        DELETE FROM recipe_parts WHERE recipe_id = $1 AND incredient_id = $2;
    ",
    )
    .bind(recipe_id)
    .bind(incredient_id)
    .execute(&*pool)
    .await
    .map_err(|e| QueryError::from(e).into())?;

    update_recipe_cached_data(recipe_id, pool).await?;

    Ok(())
}

pub async fn calculate_recipe_cached_data(
    recipe_id: i32,
    pool: &Pool<Postgres>,
) -> Result<RecipeCacheData, potion::Error> {
    let data: Option<RecipeCacheData> = sqlx::query_as("
        SELECT COUNT(e1) AS incredient_count,
            bool_and(e1.apc) AS available_alko,
            bool_and(e1.sapc) AS available_superalko,

            COALESCE(SUM(e1.volume), 0) AS total_volume,

            ((SUM(e1.ethanol_min) + SUM(e1.ethanol_max)) / 2) / 17.7 AS standard_servings,
            (( SUM(e1.alko_price_min) + SUM(e1.alko_price_average) ) / 2) / COALESCE( NULLIF( ( ( ( SUM(e1.ethanol_min) + SUM(e1.ethanol_max) ) / 2) / 17.7), 0 ), 1) AS alko_price_per_serving,
            (( SUM(e1.superalko_price_average) + SUM(e1.superalko_price_min) ) / 2) / COALESCE( NULLIF( ( ( ( SUM(e1.ethanol_min) + SUM(e1.ethanol_max) ) / 2) / 17.7), 0 ), 1) AS superalko_price_per_serving,

            ( ( ( SUM(e1.ethanol_min) + SUM(e1.ethanol_max) ) / 2) / COALESCE ( NULLIF ( ( ( SUM(e1.alko_price_average) + SUM(e1.alko_price_min) ) / 2 ), 0 ), 1 ) ) / 10 as alko_aer,
            ( ( ( SUM(e1.ethanol_min) + SUM(e1.ethanol_max) ) / 2) / COALESCE ( NULLIF ( ( ( SUM(e1.superalko_price_average) + SUM(e1.superalko_price_min) ) / 2 ), 0 ), 1 ) ) / 10 as superalko_aer,

            SUM(e1.alko_price_min) AS alko_price_min,
            SUM(e1.alko_price_max) AS alko_price_max,
            (SUM(e1.alko_price_average) + SUM(e1.alko_price_min)) / 2 AS alko_price_average,

            SUM(e1.superalko_price_min) AS superalko_price_min,
            SUM(e1.superalko_price_max) AS superalko_price_max,
            (SUM(e1.superalko_price_average) + SUM(e1.superalko_price_min)) / 2 AS superalko_price_average,

            (SUM(e1.ethanol_min) / COALESCE (NULLIF ( SUM(e1.volume), 0), 1)) * 100 AS abv_min,
            ( ( (SUM(e1.ethanol_min) / COALESCE (NULLIF ( SUM(e1.volume), 0), 1 )) + ( SUM(e1.ethanol_max) / COALESCE (NULLIF ( SUM(e1.volume), 0), 1) ) ) / 2) * 100 AS abv_average,
            (SUM(e1.ethanol_max) / COALESCE (NULLIF ( SUM(e1.volume), 0), 1)) * 100 AS abv_max
        FROM (
            SELECT rp.recipe_id AS id,
                rp.amount_standard AS volume,

                (d.abv_min / 100) * rp.amount_standard AS ethanol_min,
                (d.abv_max / 100) * rp.amount_standard AS ethanol_max,

                (d.alko_price_min / 1000) * rp.amount_standard as alko_price_min,
                (d.alko_price_average / 1000) * rp.amount_standard as alko_price_average,
                (d.alko_price_max / 1000) * rp.amount_standard as alko_price_max,

                (d.superalko_price_min / 1000) * rp.amount_standard as superalko_price_min,
                (d.superalko_price_average / 1000) * rp.amount_standard as superalko_price_average,
                (d.superalko_price_max / 1000) * rp.amount_standard as superalko_price_max,

                d.alko_product_count > 0 AS apc,
                d.superalko_product_count > 0 AS sapc
            FROM recipe_parts rp
            INNER JOIN drink_incredients d ON d.id = rp.incredient_id
            WHERE rp.recipe_id = $1
            GROUP BY (rp.recipe_id, d.id, rp.amount_standard, d.alko_product_count, d.superalko_product_count, d.abv_min, d.alko_price_min, d.superalko_price_min, d.abv_max, d.alko_price_max, d.superalko_price_max, d.alko_price_average, d.superalko_price_average)
        ) e1;
    ")
    .bind(recipe_id)
    .fetch_optional(&*pool).await.map_err(|e| QueryError::from(e).into())?;

    match data {
        Some(data) => Ok(data),
        None => Ok(RecipeCacheData::default()),
    }
}

pub async fn update_recipe_cached_data(
    recipe_id: i32,
    pool: &Pool<Postgres>,
) -> Result<(), potion::Error> {
    let data = calculate_recipe_cached_data(recipe_id, pool).await?;

    sqlx::query(
        "
        UPDATE drink_recipes SET
        abv_min = $1, 
        abv_max = $2, 
        abv_average = $3, 
        alko_price_min = $4, 
        alko_price_max = $5, 
        alko_price_average = $6,
        superalko_price_min = $7, 
        superalko_price_max = $8, 
        superalko_price_average = $9,
        incredient_count = $10,
        total_volume = $11,
        standard_servings = $12,
        alko_price_per_serving = $13,
        superalko_price_per_serving = $14,
        alko_aer = $15,
        superalko_aer = $16,
        available_alko = $17,
        available_superalko = $18
        WHERE recipe_id = $19
    ",
    )
    .bind(data.abv_min)
    .bind(data.abv_max)
    .bind(data.abv_average)
    .bind(data.alko_price_min)
    .bind(data.alko_price_max)
    .bind(data.alko_price_average)
    .bind(data.superalko_price_min)
    .bind(data.superalko_price_max)
    .bind(data.superalko_price_average)
    .bind(data.incredient_count)
    .bind(data.total_volume)
    .bind(data.standard_servings)
    .bind(data.alko_price_per_serving)
    .bind(data.superalko_price_per_serving)
    .bind(data.alko_aer)
    .bind(data.superalko_aer)
    .bind(data.available_alko)
    .bind(data.available_superalko)
    .bind(recipe_id)
    .execute(&*pool)
    .await
    .map_err(|e| QueryError::from(e).into())?;

    Ok(())
}

pub async fn list_incredients(pool: &Pool<Postgres>) -> Result<Vec<Incredient>, potion::Error> {
    let rows: Vec<Incredient> = sqlx::query_as("SELECT * FROM drink_incredients;")
        .fetch_all(&*pool)
        .await
        .map_err(|e| QueryError::from(e).into())?;

    Ok(rows)
}

pub async fn fetch_incredients(
    category: Option<ProductType>,
    order: Option<IncredientOrder>,
    offset: i64,
    search: String,
    pool: &Pool<Postgres>,
) -> Result<PageContext<IncredientRow>, potion::Error> {
    let order = order
        .map(|order| match order {
            IncredientOrder::Alphabetical => "name",
            IncredientOrder::AbvAsc => "abv_average",
            IncredientOrder::AbvDesc => "abv_average DESC",
            IncredientOrder::PriceSuperalkoAsc => "superalko_price_min",
            IncredientOrder::PriceSuperalkoDesc => "superalko_price_max DESC",
            IncredientOrder::PriceAlkoAsc => "alko_price_min",
            IncredientOrder::PriceAlkoDesc => "alko_price_max DESC",
        })
        .unwrap_or("name");

    let rows: Vec<IncredientRow> = match category {
        Some(category)=> {
            sqlx::query_as(&format!("SELECT d.*, COUNT(dd) OVER() FROM drink_incredients d LEFT JOIN drink_incredients dd ON dd.id = d.id WHERE d.type = $1 AND d.name ILIKE $2 ORDER BY {order} LIMIT $3 OFFSET $4"))
                .bind(category)
                .bind(search)
                .bind(INCREDIENT_COUNT_PER_PAGE)
                .bind(offset)
                .fetch_all(pool).await.map_err(|e| QueryError::from(e).into())?
        },
        None => {
            sqlx::query_as(&format!("SELECT d.*, COUNT(dd) OVER() FROM drink_incredients d LEFT JOIN drink_incredients dd ON dd.id = d.id WHERE d.name ILIKE $1 ORDER BY {order} LIMIT $2 OFFSET $3"))
                .bind(search)
                .bind(INCREDIENT_COUNT_PER_PAGE)
                .bind(offset)
                .fetch_all(pool).await.map_err(|e| QueryError::from(e).into())?
        },
    };

    let total_count = *&rows.get(0).map(|p| p.count).unwrap_or(0);
    let page = PageContext::from_rows(rows, total_count, INCREDIENT_COUNT_PER_PAGE, offset);
    Ok(page)
}

pub async fn create_incredient(
    category: Option<ProductType>,
    name: String,
    user_id: i32,
    pool: &Pool<Postgres>,
) -> Result<i32, potion::Error> {
    let result: (i32,) = sqlx::query_as(
        "
        INSERT INTO drink_incredients(type, author_id, name, recipe_id)
        VALUES ($1, $2, $3, NULL)
        ON CONFLICT DO NOTHING RETURNING *;
    ",
    )
    .bind(category)
    .bind(user_id)
    .bind(name)
    .fetch_one(pool)
    .await
    .map_err(|e| QueryError::from(e).into())?;

    Ok(result.0)
}

pub async fn find_incredient(
    name: &str,
    pool: &Pool<Postgres>,
) -> Result<Option<i32>, potion::Error> {
    let row: Option<(i32,)> =
        sqlx::query_as("SELECT id FROM drink_incredients WHERE LOWER(name) = LOWER($1)")
            .bind(name)
            .fetch_optional(&*pool)
            .await
            .map_err(|e| QueryError::from(e).into())?;

    Ok(row.map(|r| r.0))
}

pub async fn get_incredient(
    id: i32,
    pool: &Pool<Postgres>,
) -> Result<Option<Incredient>, potion::Error> {
    let row: Option<Incredient> = sqlx::query_as("SELECT * FROM drink_incredients WHERE id = $1")
        .bind(id)
        .fetch_optional(&*pool)
        .await
        .map_err(|e| QueryError::from(e).into())?;

    Ok(row)
}

pub async fn get_incredient_mut(
    id: i32,
    session: SessionData,
    pool: &Pool<Postgres>,
) -> Result<Incredient, potion::Error> {
    let incredient = get_incredient(id, pool).await?;

    session.authenticate(ActionType::ManageOwnIncredients)?;

    match incredient {
        Some(incredient) => match session.authenticate(ActionType::ManageAllIncredients) {
            Ok(_) => Ok(incredient),
            Err(_) => {
                if incredient.author_id != session.user_id {
                    Err(HtmlError::Unauthorized.default())
                } else {
                    Ok(incredient)
                }
            }
        },
        None => Err(HtmlError::InvalidRequest.new("No incredient exists with spcified id")),
    }
}

pub async fn get_product_filter_noname_all(
    pool: &Pool<Postgres>,
) -> Result<Vec<IngredientFilterList>, potion::Error> {
    let rows: Vec<IncredientFilterObjectNoName> = sqlx::query_as(
        "
        SELECT * FROM incredient_product_filters
    ",
    )
    .fetch_all(&*pool)
    .await
    .map_err(|e| QueryError::from(e).into())?;
    let mut hashmap: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
    rows.into_iter()
        .for_each(|x| match hashmap.get_mut(&x.incredient_id) {
            Some(v) => v.push(x.product_id),
            None => {
                hashmap.insert(x.incredient_id, vec![x.product_id]);
            }
        });
    let res = hashmap
        .into_iter()
        .map(|(k, v)| IngredientFilterList {
            ingredient_id: k,
            product_ids: v,
        })
        .collect();
    Ok(res)
}

pub async fn get_product_filter(
    pool: &Pool<Postgres>,
    incredient_id: i32,
) -> Result<Vec<IncredientFilterObject>, potion::Error> {
    let rows: Vec<IncredientFilterObject> = sqlx::query_as(
        "
        SELECT f.incredient_id AS incredient_id, f.product_id AS product_id, p.name AS product_name
        FROM incredient_product_filters f
        RIGHT JOIN products p ON p.id = product_id
        WHERE incredient_id = $1
    ",
    )
    .bind(incredient_id)
    .fetch_all(&*pool)
    .await
    .map_err(|e| QueryError::from(e).into())?;

    Ok(rows)
}

pub async fn update_incredient_info(
    id: i32,
    category: Option<ProductType>,
    name: String,
    pool: &Pool<Postgres>,
) -> Result<(), potion::Error> {
    sqlx::query("UPDATE drink_incredients SET name = $1, type = $2 WHERE id = $3")
        .bind(name)
        .bind(category)
        .bind(id)
        .execute(&*pool)
        .await
        .map_err(|e| QueryError::from(e).into())?;

    Ok(())
}

pub async fn update_incredient_price(
    id: i32,
    min: f64,
    max: f64,
    pool: &Pool<Postgres>,
) -> Result<(), potion::Error> {
    let avg = (min + max) / 2.0;

    sqlx::query(
        "
        UPDATE drink_incredients SET
        alko_price_min = $1,
        alko_price_average = $2,
        alko_price_max = $3,
        superalko_price_min = $1,
        superalko_price_average = $2,
        superalko_price_max = $3,
        alko_product_count = 1,
        superalko_product_count = 1
        WHERE id = $4
    ",
    )
    .bind(min)
    .bind(avg)
    .bind(max)
    .bind(id)
    .execute(&*pool)
    .await
    .map_err(|e| QueryError::from(e).into())?;

    Ok(())
}

pub async fn update_incredient_product_category(
    id: i32,
    category: i32,
    pool: &Pool<Postgres>,
    use_static_filter: bool,
) -> Result<(), potion::Error> {
    sqlx::query("UPDATE drink_incredients SET category = $1, use_static_filter = $2 WHERE id = $3")
        .bind(category)
        .bind(use_static_filter)
        .bind(id)
        .execute(&*pool)
        .await
        .map_err(|e| QueryError::from(e).into())?;

    Ok(())
}

pub async fn set_product_category(
    id: i32,
    subcategory: i32,
    pool: &Pool<Postgres>,
) -> Result<(), potion::Error> {
    sqlx::query("UPDATE drink_incredients SET static_filter = $1 WHERE id = $2")
        .bind(subcategory)
        .bind(id)
        .execute(&*pool)
        .await
        .map_err(|e| QueryError::from(e).into())?;

    update_incredient_cached_data(id, pool).await?;

    Ok(())
}

pub async fn insert_product_filter(
    id: i32,
    id_map: Vec<i32>,
    pool: &Pool<Postgres>,
) -> Result<(), potion::Error> {
    if id_map.len() > 0 {
        let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
            "INSERT INTO incredient_product_filters (incredient_id, product_id) ",
        );

        query_builder.push_values(id_map.iter().take(65535 / 4), |mut b, product_id| {
            b.push_bind(id).push_bind(product_id);
        });

        query_builder
            .build()
            .execute(&*pool)
            .await
            .map_err(|e| QueryError::from(e).into())?;

        update_incredient_cached_data(id, pool).await?;
    }

    Ok(())
}

pub async fn remove_product_filter(
    id: i32,
    product_id: i32,
    pool: &Pool<Postgres>,
) -> Result<(), potion::Error> {
    sqlx::query(
        "DELETE FROM incredient_product_filters WHERE product_id = $1 AND incredient_id = $2",
    )
    .bind(product_id)
    .bind(id)
    .execute(&*pool)
    .await
    .map_err(|e| QueryError::from(e).into())?;

    update_incredient_cached_data(id, pool).await?;

    Ok(())
}

pub async fn calculate_incredient_cached_data(
    incredient_id: i32,
    pool: &Pool<Postgres>,
) -> Result<IncredientCacheData, potion::Error> {
    let incredient = get_incredient(incredient_id, pool).await?;
    if incredient.is_none() {
        return Err(HtmlError::InvalidRequest.default().into());
    }

    let data: Option<IncredientCacheData> = match incredient.unwrap().static_filter {
        Some(subcategory_id) => sqlx::query_as(
            "
                SELECT 
                    COALESCE(AVG(ap.unit_price), 0) AS alko_price_average,
                    COALESCE(MAX(ap.unit_price), 0) AS alko_price_max,
                    COALESCE(min(ap.unit_price), 0) AS alko_price_min,
                
                    COALESCE(AVG(sap.unit_price), 0) AS superalko_price_average,
                    COALESCE(MAX(sap.unit_price), 0) AS superalko_price_max,
                    COALESCE(MIN(sap.unit_price), 0) AS superalko_price_min,
                
                    AVG(p.abv) AS abv_average,
                    MAX(p.abv) AS abv_max,
                    MIN(p.abv) AS abv_min,
                
                    COUNT(ap) AS alko_product_count,
                    COUNT(sap) AS superalko_product_count
                FROM products p
                LEFT JOIN products ap ON (ap.id = p.id AND ap.retailer = 'alko')
                LEFT JOIN products sap ON (sap.id = p.id AND sap.retailer = 'superalko')
                WHERE p.subcategory_id = $1
            ",
        )
        .bind(subcategory_id)
        .fetch_optional(&*pool)
        .await
        .map_err(|e| QueryError::from(e).into())?,
        None => sqlx::query_as(
            "
                SELECT 
                    COALESCE(AVG(ap.unit_price), 0) AS alko_price_average,
                    COALESCE(MAX(ap.unit_price), 0) AS alko_price_max,
                    COALESCE(min(ap.unit_price), 0) AS alko_price_min,

                    COALESCE(AVG(sap.unit_price), 0) AS superalko_price_average,
                    COALESCE(MAX(sap.unit_price), 0) AS superalko_price_max,
                    COALESCE(MIN(sap.unit_price), 0) AS superalko_price_min,

                    AVG(p.abv) AS abv_average,
                    MAX(p.abv) AS abv_max,
                    MIN(p.abv) AS abv_min,

                    COUNT(ap) AS alko_product_count,
                    COUNT(sap) AS superalko_product_count
                FROM incredient_product_filters f
                LEFT JOIN products p ON p.id = product_id
                LEFT JOIN products ap ON (ap.id = product_id AND ap.retailer = 'alko')
                LEFT JOIN products sap ON (sap.id = product_id AND sap.retailer = 'superalko')
                WHERE incredient_id = $1
                GROUP BY f.incredient_id
            ",
        )
        .bind(incredient_id)
        .fetch_optional(&*pool)
        .await
        .map_err(|e| QueryError::from(e).into())?,
    };

    match data {
        Some(data) => Ok(data),
        None => Ok(IncredientCacheData::default()),
    }
}

pub async fn update_incredient_cached_data(
    incredient_id: i32,
    pool: &Pool<Postgres>,
) -> Result<(), potion::Error> {
    let data = calculate_incredient_cached_data(incredient_id, pool).await?;

    sqlx::query(
        "
        UPDATE drink_incredients SET
        abv_min = $1, 
        abv_max = $2, 
        abv_average = $3,

        alko_price_min = $4, 
        alko_price_max = $5, 
        alko_price_average = $6,

        superalko_price_min = $7, 
        superalko_price_max = $8, 
        superalko_price_average = $9,

        alko_product_count = $10, 
        superalko_product_count = $11

        WHERE id = $12
    ",
    )
    .bind(data.abv_min)
    .bind(data.abv_max)
    .bind(data.abv_average)
    .bind(data.alko_price_min)
    .bind(data.alko_price_max)
    .bind(data.alko_price_average)
    .bind(data.superalko_price_min)
    .bind(data.superalko_price_max)
    .bind(data.superalko_price_average)
    .bind(data.alko_product_count)
    .bind(data.superalko_product_count)
    .bind(incredient_id)
    .execute(&*pool)
    .await
    .map_err(|e| QueryError::from(e).into())?;

    Ok(())
}

pub async fn get_product(id: i32, pool: &Pool<Postgres>) -> Result<Option<Product>, potion::Error> {
    let product: Option<Product> = sqlx::query_as("SELECT * FROM products WHERE id = $1")
        .bind(id)
        .fetch_optional(&*pool)
        .await
        .map_err(|e| QueryError::from(e).into())?;

    Ok(product)
}

pub async fn get_product_categories(pool: &Pool<Postgres>) -> Result<Vec<Category>, potion::Error> {
    let rows: Vec<Category> = sqlx::query_as("SELECT * FROM categories")
        .fetch_all(&*pool)
        .await
        .map_err(|e| QueryError::from(e).into())?;

    Ok(rows)
}

pub async fn get_product_subcategories(
    pool: &Pool<Postgres>,
    category_id: i32,
) -> Result<Vec<SubCategory>, potion::Error> {
    let rows: Vec<SubCategory> =
        sqlx::query_as("SELECT * FROM subcategories WHERE category_id = $1")
            .bind(category_id)
            .fetch_all(pool)
            .await
            .map_err(|e| QueryError::from(e).into())?;

    Ok(rows)
}

pub async fn get_product_category(
    category_id: i32,
    pool: &Pool<Postgres>,
) -> Result<Option<Category>, potion::Error> {
    let rows: Option<Category> = sqlx::query_as("SELECT * FROM categories WHERE id = $1")
        .bind(category_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| QueryError::from(e).into())?;

    Ok(rows)
}

pub async fn get_product_subcategory(
    subcategory_id: i32,
    pool: &Pool<Postgres>,
) -> Result<Option<SubCategory>, potion::Error> {
    let rows: Option<SubCategory> = sqlx::query_as("SELECT * FROM subcategories WHERE id = $1")
        .bind(subcategory_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| QueryError::from(e).into())?;

    Ok(rows)
}

pub async fn fetch_all_products(pool: &Pool<Postgres>) -> Result<Vec<Product>, potion::Error> {
    let rows: Vec<Product> = sqlx::query_as(
        "
                            SELECT * FROM products
                        ",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| QueryError::from(e).into())?;

    Ok(rows)
}

pub async fn fetch_products(
    search: String,
    category_id: Option<i32>,
    sub_category: Option<i32>,
    offset: i64,
    pool: &Pool<Postgres>,
) -> Result<PageContext<ProductRow>, potion::Error> {
    let rows: Vec<ProductRow> = match (category_id, sub_category) {
        (Some(category_id), Some(subcategory_id)) => {
            sqlx::query_as("
                SELECT p.id, p.name, p.href, p.img, COUNT(pp) OVER() FROM products p LEFT JOIN products pp ON pp.id = p.id WHERE p.category_id = $1 AND p.subcategory_id = $2 AND p.name ILIKE $3 LIMIT $4 OFFSET $5
            ")
                .bind(category_id)
                .bind(subcategory_id)
                .bind(search)
                .bind(PRODUCT_COUNT_PER_PAGE)
                .bind(offset)
                .fetch_all(pool).await.map_err(|e| QueryError::from(e).into())?
        },
        (Some(category_id), None) => {
            sqlx::query_as("
                SELECT p.id, p.name, p.href, p.img, COUNT(pp) OVER() FROM products p LEFT JOIN products pp ON pp.id = p.id WHERE p.category_id = $1 AND p.name ILIKE $2 LIMIT $3 OFFSET $4
            ")
                .bind(category_id)
                .bind(search)
                .bind(PRODUCT_COUNT_PER_PAGE)
                .bind(offset)
                .fetch_all(pool).await.map_err(|e| QueryError::from(e).into())?
        },
        (None, Some(subcategory_id)) => {
            sqlx::query_as("
                SELECT p.id, p.name, p.href, p.img, COUNT(pp) OVER() FROM products p LEFT JOIN products pp ON pp.id = p.id WHERE p.subcategory_id = $1 AND p.name ILIKE $2 LIMIT $3 OFFSET $4
            ")
                .bind(subcategory_id)
                .bind(search)
                .bind(PRODUCT_COUNT_PER_PAGE)
                .bind(offset)
                .fetch_all(&*pool).await.map_err(|e| QueryError::from(e).into())?
        },
        (None, None) => {
            sqlx::query_as("
                SELECT p.id, p.name, p.href, p.img, COUNT(pp) OVER() FROM products p LEFT JOIN products pp ON pp.id = p.id WHERE p.name ILIKE $1 LIMIT $2 OFFSET $3
            ")
                .bind(search)
                .bind(PRODUCT_COUNT_PER_PAGE)
                .bind(offset)
                .fetch_all(&*pool).await.map_err(|e| QueryError::from(e).into())?
        }
    };

    let total_count = *&rows.get(0).map(|p| p.count).unwrap_or(0);
    let page = PageContext::from_rows(rows, total_count, PRODUCT_COUNT_PER_PAGE, offset);

    Ok(page)
}

pub async fn is_favorite(
    id: i32,
    user_id: i32,
    pool: &Pool<Postgres>,
) -> Result<bool, potion::Error> {
    let result: Option<(i32,)> = sqlx::query_as(
        "
        SELECT drink_id FROM user_favorites WHERE drink_id = $1 AND user_id = $2
    ",
    )
    .bind(id)
    .bind(user_id)
    .fetch_optional(&*pool)
    .await
    .map_err(|e| QueryError::from(e).into())?;

    Ok(result.is_some())
}

pub async fn fetch_favorites(
    user_id: i32,
    offset: i64,
    pool: &Pool<Postgres>,
) -> Result<PageContext<RecipeRowPartial>, potion::Error> {
    let rows: Vec<RecipeRowPartial> = sqlx::query_as("
        SELECT r.*, COUNT(rr) OVER() FROM user_favorites f LEFT JOIN drink_recipes r ON r.id = f.drink_id LEFT JOIN drink_recipes rr ON rr.id = r.id WHERE f.user_id = $1 LIMIT $2 OFFSET $3
    ")
        .bind(user_id)
        .bind(RECIPE_COUNT_PER_PAGE)
        .bind(offset)
        .fetch_all(pool).await.map_err(|e| QueryError::from(e).into())?;

    let total_count = *&rows.get(0).map(|p| p.count).unwrap_or(0);
    let page = PageContext::from_rows(rows, total_count, PRODUCT_COUNT_PER_PAGE, offset);

    Ok(page)
}

pub async fn add_to_favorites(
    id: i32,
    user_id: i32,
    pool: &Pool<Postgres>,
) -> Result<(), potion::Error> {
    let result = sqlx::query("INSERT INTO user_favorites (user_id, drink_id) VALUES ($1, $2) ON CONFLICT DO NOTHING RETURNING *;")
        .bind(user_id)
        .bind(id)
        .execute(pool).await.map_err(|e| QueryError::from(e).into())?;

    if result.rows_affected() <= 0 {
        return Err(HtmlError::InvalidRequest
            .new("Recipe is already in favorites")
            .into());
    }

    sqlx::query("UPDATE drink_recipes SET favorite_count = favorite_count + 1  WHERE id = $1;")
        .bind(user_id)
        .bind(id)
        .execute(pool)
        .await
        .map_err(|e| QueryError::from(e).into())?;

    Ok(())
}

pub async fn remove_from_favorites(
    id: i32,
    user_id: i32,
    pool: &Pool<Postgres>,
) -> Result<(), potion::Error> {
    let result = sqlx::query("DELETE FROM user_favorites WHERE user_id = $1 AND drink_id = $2")
        .bind(user_id)
        .bind(id)
        .execute(pool)
        .await
        .map_err(|e| QueryError::from(e).into())?;

    if result.rows_affected() <= 0 {
        return Err(HtmlError::InvalidRequest
            .new("Recipe is not in favorites")
            .into());
    }

    sqlx::query("UPDATE drink_recipes SET favorite_count = favorite_count - 1  WHERE id = $1;")
        .bind(user_id)
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

/// Deletes a cabinet with a given id.
/// ATTENTION: DOES NOT CHECK FOR OWNERWHIP BY ITSELF
pub async fn delete_cabinet(id: i32, pool: &Pool<Postgres>) -> Result<(), potion::Error> {
    sqlx::query("DELETE FROM cabinets WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await
        .map_err(|e| QueryError::from(e).into())?;
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
        "UPDATE cabinet_products SET amount_ml = $1 WHERE cabinet_id = $2 AND product_id = $3",
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
        sqlx::query("DELETE FROM cabinet_products WHERE cabinet_id = $1 AND product_id = $2")
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
        "UPDATE cabinet_products SET usable = false WHERE cabinet_id = $1 AND product_id = $2",
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
        "UPDATE cabinet_products SET usable = true WHERE cabinet_id = $1 AND product_id = $2",
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
        "UPDATE cabinet_products SET amount_ml = $1 WHERE cabinet_id = $2 AND product_id = $3",
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
    dbg!(&token);

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

pub async fn create_tag(
    name: &str,
    pool: &Pool<Postgres>,
) -> Result<i32, potion::Error> {

    let id: (i32,) = sqlx::query_as("INSERT INTO recipe_tags (name) VALUES ($1) ON CONFLICT DO NOTHING RETURNING *")
            .bind(name)
            .fetch_one(pool)
            .await
            .map_err(|e| QueryError::from(e).into())?;

    Ok(id.0)
}

pub async fn get_tag(id: i32, pool: &Pool<Postgres>) -> Result<Option<RecipeTag>, potion::Error> {
    let list: Option<RecipeTag> = sqlx::query_as("SELECT * FROM recipe_tags WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(|e| QueryError::from(e).into())?;

    Ok(list)
}

pub async fn find_tag(name: &str, pool: &Pool<Postgres>) -> Result<Option<i32>, potion::Error> {
    let list: Option<(i32,)> = sqlx::query_as("SELECT id FROM recipe_tags WHERE name = $1")
        .bind(name)
        .fetch_optional(pool)
        .await
        .map_err(|e| QueryError::from(e).into())?;

    Ok(list.map(|tag| tag.0))
}

pub async fn list_tags(pool: &Pool<Postgres>) -> Result<Vec<RecipeTag>, potion::Error> {
    let list: Vec<RecipeTag> = sqlx::query_as("SELECT * FROM recipe_tags")
        .fetch_all(pool)
        .await
        .map_err(|e| QueryError::from(e).into())?;

    Ok(list)
}

pub async fn list_recipe_tags(
    pool: &Pool<Postgres>,
    recipe_id: i32,
) -> Result<Vec<LinkedRecipeTag>, potion::Error> {
    let list: Vec<LinkedRecipeTag> =
        sqlx::query_as("SELECT * FROM recipe_tags_map WHERE recipe_id = $1")
            .bind(recipe_id)
            .fetch_all(pool)
            .await
            .map_err(|e| QueryError::from(e).into())?;

    Ok(list)
}

pub async fn add_tag_to_recipe(
    recipe_id: i32,
    tag_id: i32,
    pool: &Pool<Postgres>,
) -> Result<(), potion::Error> {
    let tag = get_tag(tag_id, pool).await?;
    if tag.is_none() {
        return Err(HtmlError::InvalidRequest.new("Tag doesn't exists"));
    }
    let tag = tag.unwrap();

    sqlx::query("INSERT INTO recipe_tags_map (recipe_id, tag_id, tag_name) VALUES ($1, $2, $3) ON CONFLICT DO NOTHING RETURNING *")
            .bind(recipe_id)
            .bind(tag_id)
            .bind(tag.name)
            .execute(pool)
            .await
            .map_err(|e| QueryError::from(e).into())?;

    update_recipe_tag_list(recipe_id, pool).await?;

    Ok(())
}

pub async fn remove_tag_from_recipe(
    recipe_id: i32,
    tag_id: i32,
    pool: &Pool<Postgres>,
) -> Result<(), potion::Error> {
    sqlx::query("DELETE FROM recipe_tags_map WHERE recipe_id = $1 AND tag_id = $2")
        .bind(recipe_id)
        .bind(tag_id)
        .execute(pool)
        .await
        .map_err(|e| QueryError::from(e).into())?;

    update_recipe_tag_list(recipe_id, pool).await?;

    Ok(())
}

pub async fn update_recipe_tag_list(
    recipe_id: i32,
    pool: &Pool<Postgres>,
) -> Result<(), potion::Error> {
    let tags = list_recipe_tags(pool, recipe_id).await?;
    let tag_list = tags
        .iter()
        .map(|tag| tag.tag_name.to_owned())
        .collect::<Vec<String>>()
        .join("|");

    sqlx::query("UPDATE drink_recipes SET tag_list = $2 WHERE recipe_id = $1")
        .bind(recipe_id)
        .bind(tag_list)
        .execute(pool)
        .await
        .map_err(|e| QueryError::from(e).into())?;

    Ok(())
}
