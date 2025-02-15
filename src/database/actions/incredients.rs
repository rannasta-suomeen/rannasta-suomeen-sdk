use std::collections::HashMap;

use potion::{HtmlError};
use sqlx::{Pool, Postgres, QueryBuilder};

use crate::{
    authentication::permissions::ActionType, error::QueryError, pagination::PageContext, schema::{
        Incredient, IncredientColor, IncredientFilterObject, ProductOrder, ProductRow, ProductType,
        RecipeAvailability, UnitType,
    }
};

use crate::{
    jwt::SessionData,
    schema::{
        IncredientCacheData, IncredientFilterObjectNoName, IncredientOrder, IncredientRow,
        IngredientFilterList, Uuid,
    },
    INCREDIENT_COUNT_PER_PAGE,
};

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
    author: Option<i32>,
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

    let rows: Vec<IncredientRow> = match (category, author) {
        (Some(category), Some(author)) => {
            sqlx::query_as(&format!("SELECT d.*, COUNT(dd) OVER() FROM drink_incredients d LEFT JOIN drink_incredients dd ON dd.id = d.id WHERE d.type = $1 AND d.author_id = $2 AND d.name ILIKE $3 ORDER BY {order} LIMIT $4 OFFSET $5"))
                .bind(category)
                .bind(author)
                .bind(search)
                .bind(INCREDIENT_COUNT_PER_PAGE)
                .bind(offset)
                .fetch_all(pool).await.map_err(|e| QueryError::from(e).into())?
        },
        (None, Some(author)) => {
            sqlx::query_as(&format!("SELECT d.*, COUNT(dd) OVER() FROM drink_incredients d LEFT JOIN drink_incredients dd ON dd.id = d.id WHERE d.author_id = $1 AND d.name ILIKE $2 ORDER BY {order} LIMIT $3 OFFSET $4"))
                .bind(author)
                .bind(search)
                .bind(INCREDIENT_COUNT_PER_PAGE)
                .bind(offset)
                .fetch_all(pool).await.map_err(|e| QueryError::from(e).into())?
        },
        (None, None) => {
            sqlx::query_as(&format!("SELECT d.*, COUNT(dd) OVER() FROM drink_incredients d LEFT JOIN drink_incredients dd ON dd.id = d.id WHERE d.name ILIKE $1 ORDER BY {order} LIMIT $2 OFFSET $3"))
                .bind(search)
                .bind(INCREDIENT_COUNT_PER_PAGE)
                .bind(offset)
                .fetch_all(pool).await.map_err(|e| QueryError::from(e).into())?
        },
        (Some(category), None) => {
            sqlx::query_as(&format!("SELECT d.*, COUNT(dd) OVER() FROM drink_incredients d LEFT JOIN drink_incredients dd ON dd.id = d.id WHERE d.type = $1 AND d.name ILIKE $2 ORDER BY {order} LIMIT $3 OFFSET $4"))
                .bind(category)
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

pub async fn delete_incredient(id: i32, pool: &Pool<Postgres>) -> Result<(), potion::Error> {
    let mut tr = pool
        .begin()
        .await
        .map_err(|_| QueryError::new("Could not start transaction".to_owned()).into())?;

    sqlx::query("DELETE FROM incredient_product_filters WHERE incredient_id = $1")
        .bind(id)
        .execute(&mut *tr)
        .await
        .map_err(|e| QueryError::from(e).into())?;

    sqlx::query("DELETE FROM user_incredients WHERE incredient_id = $1")
        .bind(id)
        .execute(&mut *tr)
        .await
        .map_err(|e| QueryError::from(e).into())?;

    sqlx::query("DELETE FROM recipe_parts WHERE incredient_id = $1")
        .bind(id)
        .execute(&mut *tr)
        .await
        .map_err(|e| QueryError::from(e).into())?;

    sqlx::query("DELETE FROM drink_incredients WHERE id = $1")
        .bind(id)
        .execute(&mut *tr)
        .await
        .map_err(|e| QueryError::from(e).into())?;

    tr.commit()
        .await
        .map_err(|_| QueryError::new("Could not commit transaction".to_owned()).into())?;
    Ok(())
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

pub async fn get_incredient_color(
    id: i32,
    pool: &Pool<Postgres>,
) -> Result<Option<IncredientColor>, potion::Error> {
    let row: Option<IncredientColor> =
        sqlx::query_as("SELECT * FROM incredient_colors WHERE incredient_id = $1")
            .bind(id)
            .fetch_optional(&*pool)
            .await
            .map_err(|e| QueryError::from(e).into())?;

    Ok(row)
}

pub async fn set_incredient_color(
    id: i32,
    pool: &Pool<Postgres>,
    r: i32,
    g: i32,
    b: i32,
    a: i32,
) -> Result<(), potion::Error> {
    let _query = match get_incredient_color(id, pool).await? {
        Some(_color) => sqlx::query(
            "UPDATE incredient_colors SET r = $1, g = $2, b = $3, a = $4 WHERE incredient_id = $5",
        )
        .bind(r)
        .bind(g)
        .bind(b)
        .bind(a)
        .bind(id)
        .execute(&*pool)
        .await
        .map_err(|e| QueryError::from(e).into())?,
        None => sqlx::query(
            "INSERT INTO incredient_colors (incredient_id, r, g, b, a) VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(id)
        .bind(r)
        .bind(g)
        .bind(b)
        .bind(a)
        .execute(&*pool)
        .await
        .map_err(|e| QueryError::from(e).into())?,
    };

    Ok(())
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

pub async fn list_product_filter_noname(
    pool: &Pool<Postgres>,
) -> Result<Vec<IncredientFilterObjectNoName>, potion::Error> {
    let rows: Vec<IncredientFilterObjectNoName> = sqlx::query_as(
        "
        SELECT * FROM incredient_product_filters
    ",
    )
    .fetch_all(&*pool)
    .await
    .map_err(|e| QueryError::from(e).into())?;

    Ok(rows)
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

pub async fn get_incredient_author(
    id: i32,
    pool: &Pool<Postgres>,
) -> Result<Option<String>, potion::Error> {
    let row: Option<(String,)> = sqlx::query_as(
        "
        SELECT u.username 
        FROM drink_incredients d
        INNER JOIN users u ON u.id = d.author_id
        WHERE d.id = $1
    ",
    )
    .bind(id)
    .fetch_optional(&*pool)
    .await
    .map_err(|e| QueryError::from(e).into())?;

    Ok(row.map(|x| x.0))
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

pub async fn fetch_product_filter(
    pool: &Pool<Postgres>,
    incredient_id: i32,
    availability: Option<RecipeAvailability>,
    order: Option<ProductOrder>,
    offset: i64,
) -> Result<PageContext<ProductRow>, potion::Error> {
    let availability = availability
        .map(|availability| match availability {
            RecipeAvailability::Any => "",
            RecipeAvailability::Alko => "AND p.retailer = 'alko'",
            RecipeAvailability::Superalko => "AND p.retailer = 'superalko'",
        })
        .unwrap_or("");

    let order = order
        .map(|order| match order {
            ProductOrder::Alphabetical => "name",
            ProductOrder::PriceAsc => "p.price ASC",
            ProductOrder::PriceDesc => "p.price DESC",
            ProductOrder::UnitPriceAsc => "p.unit_price ASC",
            ProductOrder::UnitPriceDesc => "p.unit_price DESC",
            ProductOrder::AerAsc => "p.aer ASC",
            ProductOrder::AerDesc => "p.aer DESC",
        })
        .unwrap_or("name");

    let rows: Vec<ProductRow> = sqlx::query_as(&format!(
        "
        SELECT p.*, COUNT(pp) OVER()
        FROM incredient_product_filters f
        RIGHT JOIN products p ON p.id = f.product_id
        RIGHT JOIN products pp ON pp.id = p.id
        WHERE f.incredient_id = $1 {availability}
        ORDER BY {order}
    "
    ))
    .bind(incredient_id)
    .fetch_all(&*pool)
    .await
    .map_err(|e| QueryError::from(e).into())?;

    let total_count = *&rows.get(0).map(|p| p.count).unwrap_or(0);
    let page = PageContext::from_rows(rows, total_count, INCREDIENT_COUNT_PER_PAGE, offset);

    Ok(page)
}

pub async fn update_incredient_info(
    id: i32,
    category: Option<ProductType>,
    name: String,
    unit: UnitType,
    pool: &Pool<Postgres>,
) -> Result<(), potion::Error> {
    sqlx::query("UPDATE drink_incredients SET name = $1, type = $2, unit = $3 WHERE id = $4")
        .bind(name)
        .bind(category)
        .bind(unit)
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

pub async fn update_incredient_static_filter(
    id: i32,
    category: i32,
    pool: &Pool<Postgres>,
    use_static_filter: bool,
) -> Result<(), potion::Error> {
    sqlx::query("UPDATE drink_incredients SET category = $1, use_static_filter = $2, use_static_filter_c = $2, static_filter_c = $1 WHERE id = $3")
        .bind(category)
        .bind(use_static_filter)
        .bind(id)
        .execute(&*pool)
        .await
        .map_err(|e| QueryError::from(e).into())?;

    Ok(())
}

pub async fn set_product_s_filter(
    id: i32,
    subcategory: i32,
    pool: &Pool<Postgres>,
) -> Result<(), potion::Error> {
    sqlx::query("UPDATE drink_incredients SET static_filter = $1, use_static_filter = true, use_static_filter_c = false, static_filter_c = NULL WHERE id = $2")
        .bind(subcategory)
        .bind(id)
        .execute(&*pool)
        .await
        .map_err(|e| QueryError::from(e).into())?;

    update_incredient_cached_data(id, pool).await?;

    Ok(())
}

pub async fn set_product_c_filter(
    id: i32,
    category: i32,
    pool: &Pool<Postgres>,
) -> Result<(), potion::Error> {
    sqlx::query("UPDATE drink_incredients SET static_filter_c = $1, use_static_filter_c = true WHERE id = $2")
        .bind(category)
        .bind(id)
        .execute(&*pool)
        .await
        .map_err(|e| QueryError::from(e).into())?;

    update_incredient_cached_data(id, pool).await?;

    Ok(())
}

pub async fn insert_product_filter(
    id: i32,
    id_map: &[i32],
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
    let mut incredient = incredient.unwrap();
    if !incredient.use_static_filter {
        incredient.static_filter = None;
        incredient.static_filter_c = None;
    }

    let data: Option<IncredientCacheData> =
        match (incredient.static_filter, incredient.static_filter_c) {
            (Some(subcategory_id), None) => sqlx::query_as(
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
                WHERE p.subcategory_id = $1 AND p.abv > 0;
            ",
            )
            .bind(subcategory_id)
            .fetch_optional(&*pool)
            .await
            .map_err(|e| QueryError::from(e).into())?,
            (None, Some(category_id)) | (Some(_), Some(category_id)) => sqlx::query_as(
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
                WHERE p.category_id = $1 AND p.abv > 0;
            ",
            )
            .bind(category_id)
            .fetch_optional(&*pool)
            .await
            .map_err(|e| QueryError::from(e).into())?,
            (None, None) => sqlx::query_as(
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
