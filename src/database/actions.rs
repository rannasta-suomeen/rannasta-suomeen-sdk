use std::sync::Arc;

use potion::HtmlError;
use sqlx::{Pool, Postgres, QueryBuilder};

use crate::{authentication::{cryptography::verify_password, jwt::{generate_jwt_session, JwtSessionData}}, constants::PRODUCT_COUNT_PER_PAGE};

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
    
    Ok(())
}

pub async fn remove_from_recipe(recipe_id: i32, incredient_id: i32, pool: Arc<Pool<Postgres>>) -> Result<(), potion::Error> {
    sqlx::query("
        DELETE FROM recipe_parts WHERE recipe_id = $1 AND incredient_id = $2;
    ")
        .bind(recipe_id)
        .bind(incredient_id)
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

pub async fn get_incredient(pool: Arc<Pool<Postgres>>, id: i32) -> Result<Option<Incredient>, potion::Error> {
    let row: Option<Incredient> = sqlx::query_as("SELECT * FROM drink_incredients WHERE id = $1")
    .bind(id)
    .fetch_optional(&*pool).await.map_err(|e| QueryError::from(e).into())?;

    Ok(row)
}

pub async fn fetch_incredient(session: JwtSessionData, pool: Arc<Pool<Postgres>>, id: i32) -> Result<Incredient, potion::Error> {
    let incredient = get_incredient(pool.clone(), id).await?;

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
        alko_product_count = 1
        WHERE id = $4
    ")
        .bind(min)
        .bind(avg)
        .bind(max)
        .bind(id)
        .execute(&*pool).await.map_err(|e| QueryError::from(e).into())?;
    
    Ok(())
}

pub async fn update_incredient_product_category(id: i32, category: Option<i32>, pool: Arc<Pool<Postgres>>) -> Result<(), potion::Error> {
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
        
        /*
        match update_cached_data(pool, id).await {
            Ok(_) => {},
            Err(e) => panic!("{e}"),
        }
        */
    }
    
    Ok(())
}

pub async fn remove_product_filter(id: i32, product_id: i32, pool: Arc<Pool<Postgres>>) -> Result<(), potion::Error> {
    sqlx::query("DELETE FROM incredient_product_filters WHERE product_id = $1 AND incredient_id = $2")
        .bind(product_id)
        .bind(id)
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