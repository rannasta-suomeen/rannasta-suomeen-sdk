use std::collections::HashMap;

use sqlx::{query, query_as, Pool, Postgres};

use crate::{
    error::QueryError,
    schema::{Product, ProductBestDealEntry, ProductPriceHistoryEntry, Retailer},
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

// TODO make use of price history in the future
pub async fn list_best_deals(
    pool: &Pool<Postgres>,
) -> Result<Vec<(Retailer, Vec<(String, ProductBestDealEntry)>)>, potion::Error> {
    let deals: Vec<ProductBestDealEntry> = query_as("
        WITH map AS (
            WITH categories AS (
                SELECT p.category_id, r.unnest as retailer
                FROM products p
                JOIN LATERAL (SELECT * FROM unnest(enum_range(NULL::retailer))) r ON true
                GROUP BY (p.category_id, r.unnest)
            ) SELECT c.category_id, p.id
            FROM categories c
            JOIN LATERAL (
                SELECT p.id
                FROM products p
                WHERE p.category_id = c.category_id AND p.retailer = c.retailer AND p.currently_available
                ORDER BY p.aer DESC
                LIMIT 1
            ) p ON true
        ) SELECT
        p.name,
        c.name as category,
        p.retailer,
        p.id,
        p.href,
        p.price,
        p.unit_price,
        p.abv,
        p.aer,
        P.volume
        FROM map m
        JOIN products p ON m.id = p.id
        JOIN categories c ON m.category_id = c.id;
    ").fetch_all(pool).await.map_err(|e| QueryError::from(e).into())?;

    let map: HashMap<Retailer, Vec<(String, ProductBestDealEntry)>> =
        deals.into_iter().fold(HashMap::new(), |mut map, e| {
            match map.get_mut(&e.retailer) {
                Some(l) => l.push((e.category.clone(), e)),
                None => {
                    map.insert(e.retailer.clone(), vec![(e.category.clone(), e)]);
                }
            }
            map
        });

    let mut deals: Vec<(Retailer, Vec<(String, ProductBestDealEntry)>)> = map.into_iter().collect();
    deals.sort_by(|a, b| b.0.cmp(&a.0));
    deals
        .iter_mut()
        .for_each(|(_, l)| l.sort_by(|a, b| b.0.cmp(&a.0)));

    Ok(deals)
}
