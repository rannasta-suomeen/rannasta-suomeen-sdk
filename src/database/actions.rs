use std::sync::Arc;

use potion::HtmlError;
use sqlx::{Pool, Postgres, QueryBuilder};

use crate::{authentication::{cryptography::verify_password, jwt::{generate_jwt_session, JwtSessionData}}, constants::PRODUCT_COUNT_PER_PAGE, schema::{IncredientCacheData, RecipeCacheData}};

use super::{error::QueryError, schema::{Category, Incredient, IncredientFilterObject, Product, ProductType, Recipe, RecipePart, RecipeType, SubCategory, UnitType, User}};



pub async fn get_user(pool: Arc<Pool<Postgres>>, username: String) -> Result<Option<User>, potion::Error> {
    let row: Option<User> = sqlx::query_as("SELECT * FROM users WHERE username = $1")
    .bind(username)
    .fetch_optional(&*pool).await.map_err(|e| QueryError::from(e).into())?;

    Ok(row)
}

pub async fn register_user(username: String, password: String, pool: Arc<Pool<Postgres>>) -> Result<bool, potion::Error> {
    let query = sqlx::query("
        INSERT INTO users (username, password)
        VALUES ($1, $2)
        ON CONFLICT DO NOTHING RETURNING *;
    ")
    .bind(username)
    .bind(password)
    .execute(&*pool).await.map_err(|e| QueryError::from(e).into())?;
    
    Ok(query.rows_affected() > 0)
}

pub async fn login_user(username: String, password: String, pool: Arc<Pool<Postgres>>) -> Result<String, potion::Error> {

    let user = get_user(pool, username).await?;
    if user.is_none() {
        return Err(HtmlError::InvalidRequest.new("Invalid credentials"))
    }

    let user = user.unwrap();
    let authenticated = verify_password(password, String::from(&user.password)).map_err(|_e| panic!("!")).unwrap();
    if !authenticated {
        return Err(HtmlError::InvalidRequest.new("Invalid credentials"))
    }

    let session = generate_jwt_session(&user);
    
    Ok(session)
}


pub async fn list_recipes(pool: Arc<Pool<Postgres>>) -> Result<Vec<Recipe>, potion::Error> {
    let rows: Vec<Recipe> = sqlx::query_as("SELECT * FROM drink_recipes;")
    .fetch_all(&*pool).await.map_err(|e| QueryError::from(e).into())?;

    Ok(rows)
}

pub async fn list_recipe_parts(pool: Arc<Pool<Postgres>>, recipe_id: i32) -> Result<Vec<RecipePart>, potion::Error> {
    let rows: Vec<RecipePart> = sqlx::query_as("
        SELECT rp.recipe_id AS recipe_id, d.id AS incredient_id, rp.amount AS amount, rp.unit AS unit, d.name AS name
        FROM recipe_parts rp
        INNER JOIN drink_incredients d ON d.id = rp.incredient_id
        WHERE rp.recipe_id = $1
    ")
    .bind(recipe_id)
    .fetch_all(&*pool).await.map_err(|e| QueryError::from(e).into())?;

    Ok(rows)
}

pub async fn create_recipe(category: RecipeType, user_id: i32, name: String, pool: Arc<Pool<Postgres>>) -> Result<(), potion::Error> {

    let recipe: (i32,) = sqlx::query_as("INSERT INTO recipes DEFAULT VALUES RETURNING id")
    .fetch_one(&*pool).await.map_err(|e| QueryError::from(e).into())?;

    let recipe_id = recipe.0;

    let query = sqlx::query("
        INSERT INTO drink_recipes (type, author_id, name, recipe_id)
        VALUES ($1, $2, $3, $4)
    ")
    .bind(category)
    .bind(user_id)
    .bind(name)
    .bind(recipe_id)
    .execute(&*pool).await.map_err(|e| QueryError::from(e).into())?;

    if query.rows_affected() <= 0 {
        return Err(HtmlError::InvalidRequest.new("Recipe with taht name already exists"));
    }

    Ok(())
}

pub async fn get_recipe(id: i32, pool: Arc<Pool<Postgres>>) -> Result<Option<Recipe>, potion::Error> {
    let row: Option<Recipe> = sqlx::query_as("SELECT * FROM drink_recipes WHERE id = $1")
    .bind(id)
    .fetch_optional(&*pool).await.map_err(|e| QueryError::from(e).into())?;

    Ok(row)
}

pub async fn get_recipe_mut(id: i32, session: JwtSessionData, pool: Arc<Pool<Postgres>>) -> Result<Recipe, potion::Error> {
    let recipe = get_recipe(id, pool.clone()).await?;

    match recipe {
        Some(recipe) => {
            if recipe.author_id != session.user_id {
                return Err(HtmlError::Unauthorized.default());
            }

            Ok(recipe)
        },
        None => {
            Err(HtmlError::InvalidRequest.new("No recipe exists with spcified id"))
        },
    }
}

pub async fn update_recipe_info(id: i32, name: String, category: RecipeType, info: String, pool: Arc<Pool<Postgres>>) -> Result<(), potion::Error> {
    sqlx::query("UPDATE drink_recipes SET name = $1, type = $2, info = $3 WHERE id = $4")
        .bind(name)
        .bind(category)
        .bind(info)
        .bind(id)
        .execute(&*pool).await.map_err(|e| QueryError::from(e).into())?;
    
    Ok(())
}

pub async fn add_to_recipe(recipe_id: i32, base: i32, unit: UnitType, amount: i32, pool: Arc<Pool<Postgres>>) -> Result<(), potion::Error> {
    let amount_ml = unit.convert(amount.into(), UnitType::Ml).1;

    sqlx::query("
        INSERT INTO recipe_parts (recipe_id, incredient_id, amount, amount_standard, unit) 
        VALUES ($1, $2, $3, $4, $5)
        ON CONFLICT (recipe_id, incredient_id) DO UPDATE
        SET amount = $3, amount_standard = $4, unit = $5;
    ")
        .bind(recipe_id)
        .bind(base)
        .bind(amount)
        .bind(amount_ml)
        .bind(unit)
        .execute(&*pool).await.map_err(|e| QueryError::from(e).into())?;

    update_recipe_cached_data(recipe_id, pool).await?;
    
    Ok(())
}

pub async fn remove_from_recipe(recipe_id: i32, incredient_id: i32, pool: Arc<Pool<Postgres>>) -> Result<(), potion::Error> {
    sqlx::query("
        DELETE FROM recipe_parts WHERE recipe_id = $1 AND incredient_id = $2;
    ")
        .bind(recipe_id)
        .bind(incredient_id)
        .execute(&*pool).await.map_err(|e| QueryError::from(e).into())?;

    update_recipe_cached_data(recipe_id, pool).await?;
    
    Ok(())
}

pub async fn calculate_recipe_cached_data(recipe_id: i32, pool: Arc<Pool<Postgres>>) -> Result<RecipeCacheData, potion::Error> {
    let data: Option<RecipeCacheData> = sqlx::query_as("
        SELECT COUNT(e1) AS incredient_count,
            bool_and(e1.apc) AS available_alko,
            bool_and(e1.sapc) AS available_superalko,

            SUM(e1.volume) AS total_volume,

            ((SUM(e1.ethanol_min) + SUM(e1.ethanol_max)) / 2) / 17.7 AS standard_servings,
            (( SUM(e1.alko_price_average) + SUM(e1.alko_price_min) + SUM(e1.superalko_price_average) + SUM(e1.alko_price_average) ) / 4) / COALESCE( NULLIF( ( ( ( SUM(e1.ethanol_min) + SUM(e1.ethanol_max) ) / 2) / 17.7), 0 ), 1) AS price_per_serving,

            SUM(e1.alko_price_min) AS alko_price_min,
            SUM(e1.alko_price_max) AS alko_price_max,
            (SUM(e1.alko_price_average) + SUM(e1.alko_price_min)) / 2 AS alko_price_average,

            SUM(e1.superalko_price_min) AS superalko_price_min,
            SUM(e1.superalko_price_max) AS superalko_price_max,
            (SUM(e1.superalko_price_average) + SUM(e1.superalko_price_min)) / 2 AS superalko_price_average,

            (SUM(e1.ethanol_min) / SUM(e1.volume)) * 100 AS abv_min,
            ( ( (SUM(e1.ethanol_min) / SUM(e1.volume) ) + ( SUM(e1.ethanol_max) / SUM(e1.volume) ) ) / 2) * 100 AS abv_average,
            (SUM(e1.ethanol_max) / SUM(e1.volume)) * 100 AS abv_max
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
            GROUP BY (rp.recipe_id, rp.amount_standard, d.alko_product_count, d.superalko_product_count, d.abv_min, d.alko_price_min, d.superalko_price_min, d.abv_max, d.alko_price_max, d.superalko_price_max, d.alko_price_average, d.superalko_price_average)
        ) e1;
    ")
    .bind(recipe_id)
    .fetch_optional(&*pool).await.map_err(|e| QueryError::from(e).into())?;

    match data {
        Some(data) => Ok(data),
        None => Ok(RecipeCacheData::default()),
    }
}

pub async fn update_recipe_cached_data(recipe_id: i32, pool: Arc<Pool<Postgres>>) -> Result<(), potion::Error> {
    let data = calculate_recipe_cached_data(recipe_id, pool.clone()).await?;
    
    sqlx::query("
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
        price_per_serving = $13,
        available_alko = $14,
        available_superalko = $15
        WHERE recipe_id = $16
    ")
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
    .bind(data.price_per_serving)
    .bind(data.available_alko)
    .bind(data.available_superalko)
    .bind(recipe_id)
    .execute(&*pool).await.map_err(|e| QueryError::from(e).into())?;

    Ok(())
}

pub async fn list_incredients(pool: Arc<Pool<Postgres>>) -> Result<Vec<Incredient>, potion::Error> {
    let rows: Vec<Incredient> = sqlx::query_as("SELECT * FROM drink_incredients;")
    .fetch_all(&*pool).await.map_err(|e| QueryError::from(e).into())?;

    Ok(rows)
}

pub async fn fetch_incredients(category: Option<ProductType>, search: String, pool: Arc<Pool<Postgres>>) -> Result<Vec<Incredient>, potion::Error> {

    let rows: Vec<Incredient> = match category {
        Some(category) => {
            sqlx::query_as("SELECT * FROM drink_incredients WHERE type = $1 AND name ILIKE $2;")
                .bind(category)
                .bind(search)
                .fetch_all(&*pool).await.map_err(|e| QueryError::from(e).into())?
        },
        None => {
            sqlx::query_as("SELECT * FROM drink_incredients WHERE name ILIKE $1;")
                .bind(search)
                .fetch_all(&*pool).await.map_err(|e| QueryError::from(e).into())?
        },
    };

    Ok(rows)
}

pub async fn create_incredient(category: Option<ProductType>, name: String, user_id: i32, pool: Arc<Pool<Postgres>>) -> Result<(), potion::Error> {
    sqlx::query("
        INSERT INTO drink_incredients(type, author_id, name, recipe_id)
        VALUES ($1, $2, $3, NULL)
        ON CONFLICT DO NOTHING RETURNING *;
    ")
    .bind(category)
    .bind(user_id)
    .bind(name)
    .execute(&*pool).await.map_err(|e| QueryError::from(e).into())?;
    
    Ok(())
}

pub async fn get_incredient(id: i32, pool: Arc<Pool<Postgres>>) -> Result<Option<Incredient>, potion::Error> {
    let row: Option<Incredient> = sqlx::query_as("SELECT * FROM drink_incredients WHERE id = $1")
    .bind(id)
    .fetch_optional(&*pool).await.map_err(|e| QueryError::from(e).into())?;

    Ok(row)
}

pub async fn get_incredient_mut(id: i32, session: JwtSessionData, pool: Arc<Pool<Postgres>>) -> Result<Incredient, potion::Error> {
    let incredient = get_incredient(id, pool.clone()).await?;

    match incredient {
        Some(incredient) => {
            if incredient.author_id != session.user_id {
                return Err(HtmlError::Unauthorized.default());
            }

            Ok(incredient)
        },
        None => {
            Err(HtmlError::InvalidRequest.new("No incredient exists with spcified id"))
        },
    }
}

pub async fn get_product_filter(pool: Arc<Pool<Postgres>>, incredient_id: i32) -> Result<Vec<IncredientFilterObject>, potion::Error> {
    let rows: Vec<IncredientFilterObject> = sqlx::query_as("
        SELECT f.incredient_id AS incredient_id, f.product_id AS product_id, p.name AS product_name
        FROM incredient_product_filters f
        RIGHT JOIN products p ON p.id = product_id
        WHERE incredient_id = $1
    ")
    .bind(incredient_id)
    .fetch_all(&*pool).await.map_err(|e| QueryError::from(e).into())?;

    Ok(rows)
}

pub async fn update_incredient_info(id: i32, category: Option<ProductType>, name: String, pool: Arc<Pool<Postgres>>) -> Result<(), potion::Error> {
    sqlx::query("UPDATE drink_incredients SET name = $1, type = $2 WHERE id = $3")
        .bind(name)
        .bind(category)
        .bind(id)
        .execute(&*pool).await.map_err(|e| QueryError::from(e).into())?;
    
    Ok(())
}

pub async fn update_incredient_price(id: i32, min: f64, max: f64, pool: Arc<Pool<Postgres>>) -> Result<(), potion::Error> {
    let avg = (min + max) / 2.0;
    
    sqlx::query("
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
    ")
        .bind(min)
        .bind(avg)
        .bind(max)
        .bind(id)
        .execute(&*pool).await.map_err(|e| QueryError::from(e).into())?;
    
    Ok(())
}

pub async fn update_incredient_product_category(id: i32, category: i32, pool: Arc<Pool<Postgres>>) -> Result<(), potion::Error> {
    sqlx::query("UPDATE drink_incredients SET category = $1 WHERE id = $2")
        .bind(category)
        .bind(id)
        .execute(&*pool).await.map_err(|e| QueryError::from(e).into())?;
    
    Ok(())
}

pub async fn insert_product_filter(id: i32, id_map: Vec<i32>, pool: Arc<Pool<Postgres>>) -> Result<(), potion::Error> {
    if id_map.len() > 0 {
        let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
            "INSERT INTO incredient_product_filters (incredient_id, product_id) "
        );
    
        query_builder.push_values(id_map.iter().take(65535 / 4), |mut b, product_id| {
            b.push_bind(id)
                .push_bind(product_id);
        });
    
        query_builder.build().execute(&*pool).await.map_err(|e| QueryError::from(e).into())?;
        
        update_incredient_cached_data(id, pool).await?;
    }
    
    Ok(())
}

pub async fn remove_product_filter(id: i32, product_id: i32, pool: Arc<Pool<Postgres>>) -> Result<(), potion::Error> {
    sqlx::query("DELETE FROM incredient_product_filters WHERE product_id = $1 AND incredient_id = $2")
        .bind(product_id)
        .bind(id)
        .execute(&*pool).await.map_err(|e| QueryError::from(e).into())?;

    update_incredient_cached_data(id, pool).await?;
    
    Ok(())
}

pub async fn calculate_incredient_cached_data(incredient_id: i32, pool: Arc<Pool<Postgres>>) -> Result<IncredientCacheData, potion::Error> {
    let data: Option<IncredientCacheData> = sqlx::query_as("
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
    ")
    .bind(incredient_id)
    .fetch_optional(&*pool).await.map_err(|e| QueryError::from(e).into())?;

    match data {
        Some(data) => Ok(data),
        None => Ok(IncredientCacheData::default()),
    }
}

pub async fn update_incredient_cached_data(incredient_id: i32, pool: Arc<Pool<Postgres>>) -> Result<(), potion::Error> {
    let data = calculate_incredient_cached_data(incredient_id, pool.clone()).await?;
    
    sqlx::query("
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
    ")
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
    .execute(&*pool).await.map_err(|e| QueryError::from(e).into())?;

    Ok(())
}


pub async fn get_product_categories(pool: Arc<Pool<Postgres>>) -> Result<Vec<Category>, potion::Error> {
    let rows: Vec<Category> = sqlx::query_as("SELECT * FROM categories")
    .fetch_all(&*pool).await.map_err(|e| QueryError::from(e).into())?;

    Ok(rows)
}

pub async fn get_product_subcategories(pool: Arc<Pool<Postgres>>, category_id: i32) -> Result<Vec<SubCategory>, potion::Error> {
    let rows: Vec<SubCategory> = sqlx::query_as("SELECT * FROM subcategories WHERE category_id = $1")
    .bind(category_id)
    .fetch_all(&*pool).await.map_err(|e| QueryError::from(e).into())?;

    Ok(rows)
}

pub async fn fetch_products(search: String, category_id: i32, sub_category: Option<i32>, offset: i64, pool: Arc<Pool<Postgres>>) -> Result<(Vec<Product>, i64, i64), potion::Error> {

    let rows: Vec<Product> = match sub_category {
        Some(subcategory_id) => {
            sqlx::query_as("
                SELECT p.*, COUNT(pp) OVER() FROM products p LEFT JOIN products pp ON pp.id = p.id WHERE p.category_id = $1 AND p.subcategory_id = $2 AND p.name ILIKE $3 LIMIT $4 OFFSET $5
            ")
                .bind(category_id)
                .bind(subcategory_id)
                .bind(search)
                .bind(PRODUCT_COUNT_PER_PAGE)
                .bind(offset)
                .fetch_all(&*pool).await.map_err(|e| QueryError::from(e).into())?
        },
        None => {
            sqlx::query_as("
                SELECT p.*, COUNT(pp) OVER() FROM products p LEFT JOIN products pp ON pp.id = p.id WHERE p.category_id = $1 AND p.name ILIKE $2 LIMIT $3 OFFSET $4
            ")
                .bind(category_id)
                .bind(search)
                .bind(PRODUCT_COUNT_PER_PAGE)
                .bind(offset)
                .fetch_all(&*pool).await.map_err(|e| QueryError::from(e).into())?
        },
    };

    let count = *&rows.get(0).map(|p| p.count).unwrap_or(0);
    let offset = (offset + PRODUCT_COUNT_PER_PAGE).min(count);

    Ok((rows, count, offset))
}