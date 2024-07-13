pub const PRODUCT_COUNT_PER_PAGE: i64 = 100;
pub const INCREDIENT_COUNT_PER_PAGE: i64 = 10;
pub const RECIPE_COUNT_PER_PAGE: i64 = 10;

pub const INCREDIENT_CATEGORIES: &[(&str, &str)] = &[
    ("light_alcohol_product", "Light alcohol product"),
    ("strong_alcohol_product", "Strong alcohol product"),
    ("mixer", "Mixer"),
    ("grocery", "Grocery"),
];

pub const RECIPE_CATEGORIES: &[(&str, &str)] = &[
    ("cocktail", "Cocktail"),
    ("shot", "Shot"),
    ("punch", "Punch"),
];

pub const INCREDIENT_ORDERS: &[(&str, &str)] = &[
    ("alphabetical", "Alphabetical"),
    ("abv_asc", "ABV (asc)"),
    ("abv_desc", "ABV (desc)"),
    ("price_superalko_asc", "Price Superalko (asc)"),
    ("price_superalko_desc", "Price Superalko (desc)"),
    ("price_alko_asc", "Price Alko (asc)"),
    ("price_alko_desc", "Price Alko (desc)"),
];

pub const RECIPE_ORDERS: &[(&str, &str)] = &[
    ("alphabetical", "Nimi"),
    ("abv_asc", "ABV (asc)"),
    ("abv_desc", "ABV (desc)"),
    ("volume_asc", "Volume (asc)"),
    ("volume_desc", "Volume (desc)"),
    ("servings_asc", "Standard servings (asc)"),
    ("servings_desc", "Standard servings (desc)"),
    ("incredient_count_asc", "Incredient count (asc)"),
    ("incredient_count_desc", "Incredient count (desc)"),
    ("price_superalko_asc", "Price Superalko (asc)"),
    ("price_superalko_desc", "Price Superalko (desc)"),
    ("price_alko_asc", "Price Alko (asc)"),
    ("price_alko_desc", "Price Alko (desc)"),
];

pub const RECIPE_AVAILABILITIES: &[(&str, &str)] = &[
    ("any", "Any"),
    ("alko", "Available in Alko"),
    ("superalko", "Available in Superalko"),
];

pub const UNITS: &[&str] = &["cl", "ml", "oz", "kpl"];
