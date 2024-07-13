use potion::TypeError;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub type Uuid = i32;

#[derive(
    Clone, Debug, PartialEq, PartialOrd, sqlx::Type, Serialize, Eq, Ord, Hash, Deserialize,
)]
#[sqlx(type_name = "user_type", rename_all = "lowercase")]
#[serde(rename_all = "snake_case")]
pub enum UserRole {
    User,
    Creator,
    Admin,
}

#[derive(Clone, Debug, PartialEq, PartialOrd, sqlx::Type, Serialize, Eq, Ord, Hash)]
#[sqlx(type_name = "product_type", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ProductType {
    LightAlcoholProduct,
    StrongAlcoholProduct,
    Common,
    Mixer,
    Grocery,
    Generated
}

impl TryFrom<Value> for ProductType {
    type Error = TypeError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value.as_str() {
            Some(value) => match value {
                "light_alcohol_product" => Ok(Self::LightAlcoholProduct),
                "strong_alcohol_product" => Ok(Self::StrongAlcoholProduct),
                "common" => Ok(Self::Common),
                "mixer" => Ok(Self::Mixer),
                "grocery" => Ok(Self::Grocery),
                _ => Err(TypeError::new("Invalid variant")),
            },
            None => return Err(TypeError::new("Failed to parse value as string")),
        }
    }
}

#[derive(Clone, Debug, PartialEq, PartialOrd, sqlx::Type, Serialize, Eq, Ord, Hash)]
#[sqlx(type_name = "drink_type", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum RecipeType {
    Cocktail,
    Shot,
    Punch,
}

impl TryFrom<Value> for RecipeType {
    type Error = TypeError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value.as_str() {
            Some(value) => match value {
                "cocktail" => Ok(Self::Cocktail),
                "shot" => Ok(Self::Shot),
                "punch" => Ok(Self::Punch),
                _ => Err(TypeError::new("Invalid variant")),
            },
            None => return Err(TypeError::new("Failed to parse value as string")),
        }
    }
}

#[derive(Clone, Debug, PartialEq, PartialOrd, sqlx::Type, Serialize, Eq, Ord, Hash)]
#[sqlx(type_name = "unit_type", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum UnitType {
    Cl,
    Ml,
    Oz,
    Kpl,
}

impl TryFrom<Value> for UnitType {
    type Error = TypeError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value.as_str() {
            Some(value) => match value {
                "cl" => Ok(Self::Cl),
                "ml" => Ok(Self::Ml),
                "oz" => Ok(Self::Oz),
                "kpl" => Ok(Self::Kpl),
                _ => Err(TypeError::new("Invalid variant")),
            },
            None => return Err(TypeError::new("Failed to parse value as string")),
        }
    }
}

impl UnitType {
    /* conversion table */
    pub fn convert(&self, value: f64, other: Self) -> (Self, f64) {
        match (&self, &other) {
            (UnitType::Cl, UnitType::Cl) => (other, value),
            (UnitType::Cl, UnitType::Ml) => (other, value * 10.),
            (UnitType::Cl, UnitType::Oz) => (other, value * 0.338140227),
            (UnitType::Cl, UnitType::Kpl) => (other, 0.),
            (UnitType::Ml, UnitType::Cl) => (other, value * 0.1),
            (UnitType::Ml, UnitType::Ml) => (other, value),
            (UnitType::Ml, UnitType::Oz) => (other, value * 0.0338140227),
            (UnitType::Ml, UnitType::Kpl) => (other, 0.),
            (UnitType::Oz, UnitType::Cl) => (other, value * 2.95735296),
            (UnitType::Oz, UnitType::Ml) => (other, value * 29.5735296),
            (UnitType::Oz, UnitType::Oz) => (other, value),
            (UnitType::Oz, UnitType::Kpl) => (other, 0.),
            (UnitType::Kpl, UnitType::Cl) => (other, 0.),
            (UnitType::Kpl, UnitType::Ml) => (other, 0.),
            (UnitType::Kpl, UnitType::Oz) => (other, 0.),
            (UnitType::Kpl, UnitType::Kpl) => (other, value),
        }
    }
}

#[derive(Clone, Debug, PartialEq, PartialOrd, sqlx::Type, Serialize, Eq, Ord, Hash)]
#[sqlx(type_name = "retailer", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum Retailer {
    Superalko,
    Alko,
}

#[derive(Clone, Debug, PartialEq, PartialOrd, sqlx::Type, Serialize, Eq, Ord, Hash)]
#[sqlx(type_name = "drink_type", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum IncredientOrder {
    Alphabetical,
    AbvAsc,
    AbvDesc,
    PriceSuperalkoAsc,
    PriceSuperalkoDesc,
    PriceAlkoAsc,
    PriceAlkoDesc,
}

impl TryFrom<Value> for IncredientOrder {
    type Error = TypeError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value.as_str() {
            Some(value) => match value {
                "alphabetical" => Ok(Self::Alphabetical),
                "abv_asc " => Ok(Self::AbvAsc),
                "abv_desc" => Ok(Self::AbvDesc),
                "price_superalko_asc" => Ok(Self::PriceSuperalkoAsc),
                "price_superalko_desc" => Ok(Self::PriceSuperalkoDesc),
                "price_alko_asc" => Ok(Self::PriceAlkoAsc),
                "price_alko_desc" => Ok(Self::PriceAlkoDesc),
                _ => Err(TypeError::new("Invalid variant")),
            },
            None => return Err(TypeError::new("Failed to parse value as string")),
        }
    }
}

#[derive(Clone, Debug, PartialEq, PartialOrd, sqlx::Type, Serialize, Eq, Ord, Hash)]
#[sqlx(type_name = "drink_type", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum RecipeOrder {
    Alphabetical,
    AbvAsc,
    AbvDesc,
    VolumeAsc,
    VolumeDesc,
    ServingsAsc,
    ServingsDesc,
    IncredientCountAsc,
    IncredientCountDesc,
    PriceSuperalkoAsc,
    PriceSuperalkoDesc,
    PriceAlkoAsc,
    PriceAlkoDesc,
}

impl TryFrom<Value> for RecipeOrder {
    type Error = TypeError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value.as_str() {
            Some(value) => match value {
                "alphabetical" => Ok(Self::Alphabetical),
                "abv_asc" => Ok(Self::AbvAsc),
                "abv_desc" => Ok(Self::AbvDesc),
                "volume_asc" => Ok(Self::VolumeAsc),
                "volume_desc" => Ok(Self::VolumeDesc),
                "servings_asc" => Ok(Self::ServingsAsc),
                "servings_desc" => Ok(Self::ServingsDesc),
                "incredient_count_asc" => Ok(Self::IncredientCountAsc),
                "incredient_count_desc" => Ok(Self::IncredientCountDesc),
                "price_superalko_asc" => Ok(Self::PriceSuperalkoAsc),
                "price_superalko_desc" => Ok(Self::PriceSuperalkoDesc),
                "price_alko_asc" => Ok(Self::PriceAlkoAsc),
                "price_alko_desc" => Ok(Self::PriceAlkoDesc),
                _ => Err(TypeError::new("Invalid variant")),
            },
            None => return Err(TypeError::new("Failed to parse value as string")),
        }
    }
}

#[derive(Clone, Debug, PartialEq, PartialOrd, sqlx::Type, Serialize, Eq, Ord, Hash)]
#[sqlx(type_name = "drink_type", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum RecipeAvailability {
    Any,
    Alko,
    Superalko,
}

impl TryFrom<Value> for RecipeAvailability {
    type Error = TypeError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value.as_str() {
            Some(value) => match value {
                "any" => Ok(Self::Any),
                "alko" => Ok(Self::Alko),
                "superalko" => Ok(Self::Superalko),
                _ => Err(TypeError::new("Invalid variant")),
            },
            None => return Err(TypeError::new("Failed to parse value as string")),
        }
    }
}

#[derive(sqlx::FromRow, Debug, Clone, Serialize)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub password: String,
    pub uid: UserRole,
}

#[derive(sqlx::FromRow, Debug, Clone, Serialize)]
pub struct Incredient {
    pub id: Uuid,
    pub r#type: ProductType,
    pub author_id: Uuid,
    pub name: String,

    pub recipe_id: Option<Uuid>,
    pub category: Option<Uuid>,

    pub abv_average: f64,
    pub abv_max: f64,
    pub abv_min: f64,

    pub alko_price_average: f64,
    pub alko_price_max: f64,
    pub alko_price_min: f64,

    pub superalko_price_average: f64,
    pub superalko_price_max: f64,
    pub superalko_price_min: f64,

    pub alko_product_count: i32,
    pub superalko_product_count: i32,

    pub use_static_filter: bool,
    pub static_filter: Option<i32>,
}

#[derive(sqlx::FromRow, Debug, Clone, Serialize)]
pub struct IncredientRow {
    pub id: Uuid,
    pub r#type: ProductType,
    pub author_id: Uuid,
    pub name: String,

    pub abv_average: f64,
    pub abv_max: f64,
    pub abv_min: f64,

    pub alko_price_average: f64,
    pub alko_price_max: f64,
    pub alko_price_min: f64,

    pub superalko_price_average: f64,
    pub superalko_price_max: f64,
    pub superalko_price_min: f64,

    pub alko_product_count: i32,
    pub superalko_product_count: i32,

    pub count: i64,
}

#[derive(sqlx::FromRow, Debug, Clone, Serialize)]
pub struct Category {
    pub id: Uuid,
    pub name: String,
}

#[derive(sqlx::FromRow, Debug, Clone, Serialize)]
pub struct SubCategory {
    pub id: Uuid,
    pub name: String,
    pub category_id: Uuid,
    pub product_count: i32,
}

#[derive(sqlx::FromRow, Debug, Clone, Serialize)]
pub struct Product {
    pub id: Uuid,
    pub name: String,
    pub href: String,
    pub price: f64,
    pub img: String,
    pub volume: f64,
    pub category_id: Uuid,
    pub subcategory_id: Uuid,

    pub abv: f64,
    pub aer: f64,
    pub unit_price: f64,

    pub checksum: String,
    pub retailer: Retailer,
}

#[derive(sqlx::FromRow, Debug, Clone, Serialize)]
pub struct ProductRow {
    pub id: Uuid,
    pub name: String,
    pub href: String,
    pub img: String,

    pub count: i64,
}

#[derive(sqlx::FromRow, Debug, Clone, Serialize)]
pub struct IncredientFilterObject {
    pub incredient_id: Uuid,
    pub product_id: Uuid,
    pub product_name: String,
}

#[derive(sqlx::FromRow, Debug, Clone, Serialize)]
pub struct IncredientFilterObjectNoName {
    pub incredient_id: Uuid,
    pub product_id: Uuid,
}

#[derive(Serialize)]
pub struct IngredientFilterList {
    pub ingredient_id: Uuid,
    pub product_ids: Vec<Uuid>,
}

#[derive(sqlx::FromRow, Debug, Default, Clone, Serialize)]
pub struct IncredientCacheData {
    pub abv_average: f64,
    pub abv_max: f64,
    pub abv_min: f64,

    pub alko_price_max: f64,
    pub alko_price_min: f64,
    pub alko_price_average: f64,

    pub superalko_price_max: f64,
    pub superalko_price_min: f64,
    pub superalko_price_average: f64,

    pub alko_product_count: i64,
    pub superalko_product_count: i64,
}

#[derive(sqlx::FromRow, Debug, Clone, Serialize)]
pub struct Recipe {
    pub id: Uuid,
    pub r#type: RecipeType,

    pub author_id: Uuid,
    pub name: String,
    pub info: String,

    pub recipe_id: Uuid,

    pub total_volume: f64,
    pub standard_servings: f64,
    pub alko_price_per_serving: f64,
    pub superalko_price_per_serving: f64,

    pub alko_aer: f64,
    pub superalko_aer: f64,

    pub abv_average: f64,
    pub abv_max: f64,
    pub abv_min: f64,

    pub alko_price_max: f64,
    pub alko_price_min: f64,
    pub alko_price_average: f64,

    pub superalko_price_max: f64,
    pub superalko_price_min: f64,
    pub superalko_price_average: f64,

    pub incredient_count: i32,
    pub favorite_count: i32,

    pub available_superalko: bool,
    pub available_alko: bool,
}

#[derive(sqlx::FromRow, Debug, Clone, Serialize)]
pub struct RecipeRowPartial {
    pub id: Uuid,
    pub r#type: RecipeType,

    pub author_id: Uuid,
    pub name: String,

    pub tag_list: String,

    pub total_volume: f64,
    pub standard_servings: f64,
    pub alko_price_per_serving: f64,
    pub superalko_price_per_serving: f64,

    pub alko_aer: f64,
    pub superalko_aer: f64,

    pub abv_average: f64,
    pub abv_max: f64,
    pub abv_min: f64,

    pub alko_price_max: f64,
    pub alko_price_min: f64,
    pub alko_price_average: f64,

    pub superalko_price_max: f64,
    pub superalko_price_min: f64,
    pub superalko_price_average: f64,

    pub incredient_count: i32,
    pub favorite_count: i32,

    pub available_superalko: bool,
    pub available_alko: bool,

    pub count: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct RecipeRow {
    pub id: Uuid,
    pub r#type: RecipeType,

    pub author_id: Uuid,
    pub name: String,

    pub tag_list: Vec<String>,

    pub total_volume: f64,
    pub standard_servings: f64,
    pub alko_price_per_serving: f64,
    pub superalko_price_per_serving: f64,

    pub alko_aer: f64,
    pub superalko_aer: f64,

    pub abv_average: f64,
    pub abv_max: f64,
    pub abv_min: f64,

    pub alko_price_max: f64,
    pub alko_price_min: f64,
    pub alko_price_average: f64,

    pub superalko_price_max: f64,
    pub superalko_price_min: f64,
    pub superalko_price_average: f64,

    pub incredient_count: i32,
    pub favorite_count: i32,

    pub available_superalko: bool,
    pub available_alko: bool,

    pub count: i64,
}

impl From<RecipeRowPartial> for RecipeRow {
    fn from(value: RecipeRowPartial) -> Self {
        Self {
            id: value.id,
            r#type: value.r#type,
            author_id: value.author_id,
            name: value.name,
            tag_list: value.tag_list.split("|").map(|s| s.to_owned()).collect(),
            total_volume: value.total_volume,
            standard_servings: value.standard_servings,
            alko_price_per_serving: value.alko_price_per_serving,
            superalko_price_per_serving: value.superalko_price_per_serving,
            alko_aer: value.alko_aer,
            superalko_aer: value.superalko_aer,
            abv_average: value.abv_average,
            abv_max: value.abv_max,
            abv_min: value.abv_min,
            alko_price_max: value.alko_price_max,
            alko_price_min: value.alko_price_min,
            alko_price_average: value.alko_price_average,
            superalko_price_max: value.superalko_price_max,
            superalko_price_min: value.superalko_price_min,
            superalko_price_average: value.superalko_price_average,
            incredient_count: value.incredient_count,
            favorite_count: value.favorite_count,
            available_superalko: value.available_superalko,
            available_alko: value.available_alko,
            count: value.count,
        }
    }
}

#[derive(sqlx::FromRow, Debug, Clone, Serialize)]
pub struct RecipePart {
    pub recipe_id: Uuid,
    pub incredient_id: Uuid,
    pub amount: i32,
    pub unit: UnitType,
    pub name: String,
}

#[derive(Serialize)]
pub struct IngredientsForDrink {
    pub recipe_id: Uuid,
    pub recipe_parts: Vec<RecipePartNoId>,
}

// PERF: Name is not a needed part, for it can be gotten elsewhere
#[derive(Serialize)]
pub struct RecipePartNoId {
    pub ingredient_id: Uuid,
    pub amount: i32,
    pub name: String,
    pub unit: UnitType,
}

impl From<RecipePart> for RecipePartNoId {
    fn from(value: RecipePart) -> Self {
        RecipePartNoId {
            ingredient_id: value.incredient_id,
            amount: value.amount,
            name: value.name,
            unit: value.unit,
        }
    }
}

#[derive(sqlx::FromRow, Debug, Default, Clone, Serialize)]
pub struct RecipeCacheData {
    pub total_volume: f64,
    pub standard_servings: f64,

    pub alko_price_per_serving: f64,
    pub superalko_price_per_serving: f64,

    pub alko_aer: f64,
    pub superalko_aer: f64,

    pub abv_average: f64,
    pub abv_max: f64,
    pub abv_min: f64,

    pub alko_price_max: f64,
    pub alko_price_min: f64,
    pub alko_price_average: f64,

    pub superalko_price_max: f64,
    pub superalko_price_min: f64,
    pub superalko_price_average: f64,

    pub incredient_count: i64,

    pub available_superalko: bool,
    pub available_alko: bool,
}

#[derive(sqlx::FromRow, Debug, Default, Clone, Serialize)]
pub struct Cabinet {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub name: String,

    pub access_key: Option<String>,
}

#[derive(sqlx::FromRow, Debug, Default, Clone, Serialize)]
pub struct CabinetMember {
    pub cabinet_id: Uuid,
    pub user_id: Uuid,
    pub user_username: String,
}

#[derive(sqlx::FromRow, Debug, Default, Clone, Serialize)]
pub struct CabinetProduct {
    pub cabinet_id: Uuid,
    pub product_id: Uuid,
    pub owner_id: Uuid,

    pub name: String,
    pub img: String,
    pub href: String,
    pub abv: f64,

    pub amount_ml: Option<i32>,
    pub usable: bool,
}

#[derive(sqlx::FromRow, Debug, Default, Clone, Serialize, PartialEq, PartialOrd)]
pub struct RecipeTag {
    pub id: Uuid,
    pub name: String,
}

#[derive(sqlx::FromRow, Debug, Default, Clone, Serialize, PartialEq, PartialOrd)]
pub struct LinkedRecipeTag {
    pub recipe_id: Uuid,
    pub tag_id: Uuid,
    pub tag_name: String,
}
