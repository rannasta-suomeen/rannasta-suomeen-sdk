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
    ("alphabetical", "Name"),
    ("abv_asc", "ABV (asc)"),
    ("abv_desc", "ABV (desc)"),
    ("servings_asc", "Standard servings (asc)"),
    ("servings_desc", "Standard servings (desc)"),
    ("aer_superalko_asc", "Aer Superalko (asc)"),
    ("aer_superalko_desc", "Aer Superalko (desc)"),
    ("aer_alko_asc", "Aer Alko (asc)"),
    ("aer_alko_desc", "Aer Alko (desc)"),
    ("price_superalko_asc", "Price Superalko (asc)"),
    ("price_superalko_desc", "Price Superalko (desc)"),
    ("price_alko_asc", "Price Alko (asc)"),
    ("price_alko_desc", "Price Alko (desc)"),
];

pub const PRODUCT_ORDERS: &[(&str, &str)] = &[
    ("alphabetical", "Name"),
    ("price_asc", "Price (asc)"),
    ("price_desc", "Price (desc)"),
    ("unit_price_asc", "Unit Price (asc)"),
    ("unit_price_desc", "Unit Price (desc)"),
    ("aer_asc", "Aer (asc)"),
    ("aer_desc", "Aer (desc)"),
];

pub const RECIPE_AVAILABILITIES: &[(&str, &str)] = &[
    ("any", "Any"),
    ("alko", "Available in Alko"),
    ("superalko", "Available in Superalko"),
];

pub const PRODUCT_AVAILABILITIES: &[(&str, &str)] = &[
    ("any", "Either"),
    ("alko", "Alko"),
    ("superalko", "Superalko"),
];

pub const UNITS: &[&str] = &["cl", "ml", "oz", "kpl"];
