use crate::{
    error::QueryError,
    schema::{LinkedRecipeTag, RecipeTag},
};

use potion::HtmlError;
use sqlx::{Pool, Postgres};

pub async fn create_tag(name: &str, pool: &Pool<Postgres>) -> Result<i32, potion::Error> {
    let id: (i32,) = sqlx::query_as(
        "INSERT INTO recipe_tags (name) VALUES ($1) ON CONFLICT DO NOTHING RETURNING *",
    )
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
