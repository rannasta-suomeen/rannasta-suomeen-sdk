use std::collections::HashMap;

use crate::{
    authentication::permissions::ActionType,
    error::QueryError,
    schema::{Recipe, RecipePart, RecipeRow, RecipeType, UnitType},
};

use crate::{
    constants::PRODUCT_COUNT_PER_PAGE,
    jwt::SessionData,
    schema::{
        IngredientsForDrink, RecipeAvailability, RecipeCacheData, RecipeOrder, RecipePartNoId,
        RecipeRowPartial, Uuid,
    },
    RECIPE_COUNT_PER_PAGE,
};
use potion::{pagination::PageContext, HtmlError};
use sqlx::{Pool, Postgres};

pub async fn fetch_recipes(
    category: Option<RecipeType>,
    order: Option<RecipeOrder>,
    availability: Option<RecipeAvailability>,
    offset: i64,
    search: String,
    author: Option<i32>,
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

    let rows: Vec<RecipeRowPartial> = match (category, author) {
        (Some(category), Some(author)) => {
            sqlx::query_as(&format!("SELECT r.*, COUNT(rr) OVER() FROM drink_recipes r LEFT JOIN drink_recipes rr ON rr.id = r.id WHERE r.type = $1 AND r.author_id = $2 AND r.name ILIKE $3 {availability} ORDER BY {order} LIMIT $4 OFFSET $5"))
                .bind(category)
                .bind(author)
                .bind(search)
                .bind(RECIPE_COUNT_PER_PAGE)
                .bind(offset)
                .fetch_all(&*pool).await.map_err(|e| QueryError::from(e).into())?
        },
        (None, Some(author)) => {
            sqlx::query_as(&format!("SELECT r.*, COUNT(rr) OVER() FROM drink_recipes r LEFT JOIN drink_recipes rr ON rr.id = r.id WHERE r.author_id = $1 AND r.name ILIKE $2 {availability} ORDER BY {order} LIMIT $3 OFFSET $4"))
                .bind(author)
                .bind(search)
                .bind(RECIPE_COUNT_PER_PAGE)
                .bind(offset)
                .fetch_all(&*pool).await.map_err(|e| QueryError::from(e).into())?
        },
        (None, None) => {
            sqlx::query_as(&format!("SELECT r.*, COUNT(rr) OVER() FROM drink_recipes r LEFT JOIN drink_recipes rr ON rr.id = r.id WHERE r.name ILIKE $1 {availability} ORDER BY {order} LIMIT $2 OFFSET $3"))
                .bind(search)
                .bind(RECIPE_COUNT_PER_PAGE)
                .bind(offset)
                .fetch_all(&*pool).await.map_err(|e| QueryError::from(e).into())?
        },
        (Some(category), None) => {
            sqlx::query_as(&format!("SELECT r.*, COUNT(rr) OVER() FROM drink_recipes r LEFT JOIN drink_recipes rr ON rr.id = r.id WHERE r.type = $1 AND r.name ILIKE $2 {availability} ORDER BY {order} LIMIT $3 OFFSET $4"))
                .bind(category)
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

pub async fn get_recipe_author(
    id: i32,
    pool: &Pool<Postgres>,
) -> Result<Option<String>, potion::Error> {
    let row: Option<(String,)> = sqlx::query_as(
        "
        SELECT u.username 
        FROM drink_recipes r
        INNER JOIN users u ON u.id = r.author_id
        WHERE r.id = $1
    ",
    )
    .bind(id)
    .fetch_optional(&*pool)
    .await
    .map_err(|e| QueryError::from(e).into())?;

    Ok(row.map(|x| x.0))
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
