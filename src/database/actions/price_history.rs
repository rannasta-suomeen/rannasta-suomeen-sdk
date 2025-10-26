use potion::HtmlError;
use sqlx::{query, query_as, Pool, Postgres};

use crate::{
    actions::{fetch_all_products, get_product},
    error::QueryError,
    schema::{Product, ProductPriceHistoryEntry},
};

pub async fn get_price_history(
    id: i32,
    pool: &Pool<Postgres>,
) -> Result<Vec<ProductPriceHistoryEntry>, potion::Error> {
    let history: Vec<ProductPriceHistoryEntry> =
        query_as("SELECT * FROM product_price_history WHERE product_id = $1")
            .bind(id)
            .fetch_all(pool)
            .await
            .map_err(|e| QueryError::from(e).into())?;

    Ok(history)
}

pub async fn upsert_price_history(
    product: &Product,
    pool: &Pool<Postgres>,
) -> Result<(), potion::Error> {
    let history = get_price_history(product.id, pool).await?;

    match history.last() {
        Some(entry) => {
            if entry.price == product.price {
                update_price_entry(&product, pool).await?;
            } else {
                insert_price_entry(&product, pool).await?;
            }
        }
        None => {
            insert_price_entry(&product, pool).await?;
        }
    }

    Ok(())
}

async fn insert_price_entry(product: &Product, pool: &Pool<Postgres>) -> Result<(), potion::Error> {
    let _query = query("INSERT INTO product_price_history (product_id, price) VALUES ($1, $2)")
        .bind(product.id)
        .bind(product.price)
        .execute(pool)
        .await
        .map_err(|e| QueryError::from(e).into())?;

    Ok(())
}

async fn update_price_entry(product: &Product, pool: &Pool<Postgres>) -> Result<(), potion::Error> {
    let _query = query("UPDATE product_price_history SET last_timestamp = (NOW() at time zone 'utc') WHERE product_id = $1 AND price = $2")
        .bind(product.id)
        .bind(product.price)
        .execute(pool)
        .await
        .map_err(|e| QueryError::from(e).into())?;

    Ok(())
}
