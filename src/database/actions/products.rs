use crate::{
    error::QueryError,
    schema::{Category, ProductOrder, SubCategory},
};

use crate::{
    constants::PRODUCT_COUNT_PER_PAGE,
    schema::{Product, ProductRow, RecipeAvailability},
};
use potion::pagination::PageContext;
use sqlx::{Pool, Postgres};

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

pub async fn list_product_subcategories(
    pool: &Pool<Postgres>,
) -> Result<Vec<SubCategory>, potion::Error> {
    let rows: Vec<SubCategory> = sqlx::query_as("SELECT * FROM subcategories")
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
    order: Option<ProductOrder>,
    availability: Option<RecipeAvailability>,
    offset: i64,
    pool: &Pool<Postgres>,
) -> Result<PageContext<ProductRow>, potion::Error> {
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

    let availability = availability
        .map(|availability| match availability {
            RecipeAvailability::Any => "",
            RecipeAvailability::Alko => "AND p.retailer = 'alko'",
            RecipeAvailability::Superalko => "AND p.retailer = 'superalko'",
        })
        .unwrap_or("");

    let rows: Vec<ProductRow> = match (category_id, sub_category) {
        (Some(category_id), Some(subcategory_id)) => {
            sqlx::query_as(&format!("
                SELECT p.id, p.name, p.href, p.img, p.retailer, p.unit_price, p.price, p.abv, p.volume, p.aer, COUNT(pp) OVER() FROM products p LEFT JOIN products pp ON pp.id = p.id WHERE p.category_id = $1 AND p.subcategory_id = $2 AND p.name ILIKE $3 {} ORDER BY {} LIMIT $4 OFFSET $5
            ", availability, order))
                .bind(category_id)
                .bind(subcategory_id)
                .bind(search)
                .bind(PRODUCT_COUNT_PER_PAGE)
                .bind(offset)
                .fetch_all(pool).await.map_err(|e| QueryError::from(e).into())?
        },
        (Some(category_id), None) => {
            sqlx::query_as(&format!("
                SELECT p.id, p.name, p.href, p.img, p.retailer, p.unit_price, p.price, p.abv, p.volume, p.aer, COUNT(pp) OVER() FROM products p LEFT JOIN products pp ON pp.id = p.id WHERE p.category_id = $1 AND p.name ILIKE $2 {} ORDER BY {} LIMIT $3 OFFSET $4
            ", availability, order))
                .bind(category_id)
                .bind(search)
                .bind(PRODUCT_COUNT_PER_PAGE)
                .bind(offset)
                .fetch_all(pool).await.map_err(|e| QueryError::from(e).into())?
        },
        (None, Some(subcategory_id)) => {
            sqlx::query_as(&format!("
                SELECT p.id, p.name, p.href, p.img, p.retailer, p.unit_price, p.price, p.abv, p.volume, p.aer, COUNT(pp) OVER() FROM products p LEFT JOIN products pp ON pp.id = p.id WHERE p.subcategory_id = $1 AND p.name ILIKE $2 {} ORDER BY {} LIMIT $3 OFFSET $4
            ", availability, order))
                .bind(subcategory_id)
                .bind(search)
                .bind(PRODUCT_COUNT_PER_PAGE)
                .bind(offset)
                .fetch_all(&*pool).await.map_err(|e| QueryError::from(e).into())?
        },
        (None, None) => {
            sqlx::query_as(&format!("
                SELECT p.id, p.name, p.href, p.img, p.retailer, p.unit_price, p.price, p.abv, p.volume, p.aer, COUNT(pp) OVER() FROM products p LEFT JOIN products pp ON pp.id = p.id WHERE p.name ILIKE $1 {} ORDER BY {} LIMIT $2 OFFSET $3
            ", availability, order))
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
