pub const PRODUCT_COUNT_PER_PAGE: i64 = 100;
pub const INCREDIENT_COUNT_PER_PAGE: i64 = 10;
pub const RECIPE_COUNT_PER_PAGE: i64 = 10;

pub const INCREDIENT_CATEGORIES: &[(&str, &str)] = &[
    ("light_alcohol_product", "Mieto alkoholijuoma"),
    ("strong_alcohol_product", "Väkevä alkoholijuoma"),
    ("common", "Yleinen elintarvike"),
    ("mixer", "Mixeri"),
    ("grocery", "Elintarvike"),
];

pub const RECIPE_CATEGORIES: &[(&str, &str)] = &[
    ("cocktail", "Drinkki"),
    ("shot", "Shotti"),
    ("punch", "Booli"),
];

pub const INCREDIENT_ORDERS: &[(&str, &str)] = &[
    ("alphabetical", "Nimi"),
    ("abv_asc", "Alkoholipitoisuus (pienin)"),
    ("abv_desc", "Alkoholipitoisuus (suurin)"),
    ("price_superalko_asc", "Litrahinta Superalko (pienin)"),
    ("price_superalko_desc", "Litrahinta Superalko (suurin)"),
    ("price_alko_asc", "Litrahinta Alko (pienin)"),
    ("price_alko_desc", "Litrahinta Alko (suurin)"),
];

pub const RECIPE_ORDERS: &[(&str, &str)] = &[
    ("alphabetical", "Nimi"),
    ("abv_asc", "Alkoholipitoisuus (pienin)"),
    ("abv_desc", "Alkoholipitoisuus (suurin)"),
    ("volume_asc", "Kokonaistilavuus (pienin)"),
    ("volume_desc", "Kokonaistilavuus (suurin)"),
    ("servings_asc", "Annosten lukumäärä (vähiten)"),
    ("servings_desc", "Annosten lukumäärä (eniten)"),
    ("incredient_count_asc", "Aineosien monipuolisuus (vähiten)"),
    ("incredient_count_desc", "Aineosien monipuolisuus (eniten)"),
    ("price_superalko_asc", "Hinta Superalko (pienin)"),
    ("price_superalko_desc", "Hinta Superalko (suurin)"),
    ("price_alko_asc", "Hinta Alko (pienin)"),
    ("price_alko_desc", "Hinta Alko (suurin)"),
];

pub const RECIPE_AVAILABILITIES: &[(&str, &str)] = &[
    ("any", "Kaikki"),
    ("alko", "Ainoastaan Alkosta"),
    ("superalko", "Ainoastaan Superalkosta"),
];

pub const UNITS: &[&str] = &["cl", "ml", "oz", "kpl"];
