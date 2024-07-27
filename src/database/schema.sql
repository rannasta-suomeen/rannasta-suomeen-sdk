DROP TABLE IF EXISTS users CASCADE;
DROP TABLE IF EXISTS drink_recipes CASCADE;
DROP TABLE IF EXISTS recipe_tags CASCADE;
DROP TABLE IF EXISTS recipe_tags_map CASCADE;
DROP TABLE IF EXISTS drink_incredients CASCADE;
DROP TABLE IF EXISTS recipes CASCADE;
DROP TABLE IF EXISTS recipe_parts CASCADE;

DROP TABLE IF EXISTS categories CASCADE;
DROP TABLE IF EXISTS subcategories CASCADE;
DROP TABLE IF EXISTS products CASCADE;
DROP TABLE IF EXISTS user_incredients CASCADE;
DROP TABLE IF EXISTS user_favorites CASCADE;

DROP TABLE IF EXISTS recipe_tags CASCADE;
DROP TABLE IF EXISTS recipe_tags CASCADE;
DROP TABLE IF EXISTS cabinets CASCADE;
DROP TABLE IF EXISTS shared_cabinets CASCADE;
DROP TABLE IF EXISTS cabinet_products CASCADE;

DROP TABLE IF EXISTS incredient_product_filters CASCADE;

DROP TYPE IF EXISTS user_type CASCADE;
DROP TYPE IF EXISTS product_type CASCADE;
DROP TYPE IF EXISTS drink_type CASCADE;
DROP TYPE IF EXISTS unit_type CASCADE;
DROP TYPE IF EXISTS retailer CASCADE;



CREATE TYPE user_type AS ENUM ('user', 'creator', 'admin');
CREATE TYPE product_type AS ENUM ( 'light_alcohol_product', 'strong_alcohol_product', 'common', 'mixer', 'grocery', 'generated');
CREATE TYPE drink_type AS ENUM ( 'cocktail', 'shot', 'punch', 'generated' );
CREATE TYPE unit_type AS ENUM ( 'oz', 'cl', 'ml', 'tl', 'dash', 'kpl' );
CREATE TYPE retailer AS ENUM ('superalko', 'alko');

/* Users */
CREATE TABLE users (
    id SERIAL PRIMARY KEY NOT NULL,
    uid user_type NOT NULL DEFAULT 'user',
    username TEXT UNIQUE NOT NULL,
    password TEXT NOT NULL
);


/* Recipes and Incredients */

CREATE TABLE recipes (
    id SERIAL PRIMARY KEY NOT NULL
);

CREATE TABLE recipe_tags (
    id SERIAL PRIMARY KEY NOT NULL,
    name TEXT UNIQUE NOT NULL
);

CREATE TABLE drink_recipes (
    id SERIAL PRIMARY KEY NOT NULL,
    type drink_type NOT NULL,

    author_id INTEGER NOT NULL,
    name TEXT UNIQUE NOT NULL,
    info TEXT NOT NULL DEFAULT '',

    tag_list TEXT NOT NULL DEFAULT '',

    recipe_id SERIAL NOT NULL,

    total_volume FLOAT NOT NULL DEFAULT 0.0,

    standard_servings FLOAT NOT NULL DEFAULT 0.0,
    alko_price_per_serving FLOAT NOT NULL DEFAULT 0.0,
    superalko_price_per_serving FLOAT NOT NULL DEFAULT 0.0,

    alko_aer FLOAT NOT NULL DEFAULT 0,
    superalko_aer FLOAT NOT NULL DEFAULT 0,

    abv_min FLOAT NOT NULL DEFAULT 0.0,
    abv_max FLOAT NOT NULL DEFAULT 0.0,
    abv_average FLOAT NOT NULL DEFAULT 0.0,

    alko_price_min FLOAT NOT NULL DEFAULT 0.0,
    alko_price_max FLOAT NOT NULL DEFAULT 0.0,
    alko_price_average FLOAT NOT NULL DEFAULT 0.0,

    superalko_price_min FLOAT NOT NULL DEFAULT 0.0,
    superalko_price_max FLOAT NOT NULL DEFAULT 0.0,
    superalko_price_average FLOAT NOT NULL DEFAULT 0.0,

    incredient_count INTEGER NOT NULL DEFAULT 0,
    favorite_count INTEGER NOT NULL DEFAULT 0,

    available_superalko BOOLEAN NOT NULL DEFAULT false,
    available_alko BOOLEAN NOT NULL DEFAULT false,

    FOREIGN KEY (author_id) REFERENCES users (id),
    FOREIGN KEY (recipe_id) REFERENCES recipes (id)
);

CREATE TABLE recipe_tags_map(
    recipe_id SERIAL NOT NULL,
    tag_id SERIAL NOT NULL,
    tag_name TEXT NOT NULL,

    FOREIGN KEY (recipe_id) REFERENCES drink_recipes (id),
    FOREIGN KEY (tag_id) REFERENCES recipe_tags (id),
    PRIMARY KEY (recipe_id, tag_id)
);

CREATE TABLE drink_incredients (
    id SERIAL PRIMARY KEY NOT NULL,
    type product_type NOT NULL,

    author_id INTEGER NOT NULL,
    name TEXT UNIQUE NOT NULL,

    recipe_id INTEGER NULL DEFAULT NULL,
    category INT NULL DEFAULT NULL,

    abv_min FLOAT NOT NULL DEFAULT 0.0,
    abv_max FLOAT NOT NULL DEFAULT 0.0,
    abv_average FLOAT NOT NULL DEFAULT 0.0,

    alko_price_min FLOAT NOT NULL DEFAULT 0.0,
    alko_price_max FLOAT NOT NULL DEFAULT 0.0,
    alko_price_average FLOAT NOT NULL DEFAULT 0.0,

    superalko_price_min FLOAT NOT NULL DEFAULT 0.0,
    superalko_price_max FLOAT NOT NULL DEFAULT 0.0,
    superalko_price_average FLOAT NOT NULL DEFAULT 0.0,

    alko_product_count INTEGER NOT NULL DEFAULT 0,
    superalko_product_count INTEGER NOT NULL DEFAULT 0,

    use_static_filter BOOLEAN DEFAULT false,
    static_filter INTEGER NULL DEFAULT NULL,

    FOREIGN KEY (author_id) REFERENCES users (id),
    FOREIGN KEY (recipe_id) REFERENCES recipes (id)
);

CREATE TABLE recipe_parts (
    recipe_id SERIAL NOT NULL,
    incredient_id INTEGER NOT NULL,

    amount INTEGER NOT NULL,
    amount_standard FLOAT NOT NULL,

    unit unit_type NOT NULL,

    FOREIGN KEY (recipe_id) REFERENCES recipes (id),
    FOREIGN KEY (incredient_id) REFERENCES drink_incredients (id),
    PRIMARY KEY (recipe_id, incredient_id)
);

/*  products */

CREATE TABLE categories (
    id SERIAL NOT NULL PRIMARY KEY,
    name TEXT UNIQUE NOT NULL
);

CREATE TABLE subcategories (
    id SERIAL NOT NULL PRIMARY KEY,
    name TEXT UNIQUE NOT NULL,
    category_id SERIAL NOT NULL,
    product_count INT NOT NULL DEFAULT 0,

    FOREIGN KEY (category_id) REFERENCES categories (id)
);

CREATE TABLE products (
    id SERIAL NOT NULL PRIMARY KEY,

    name TEXT NOT NULL,
    href UNIQUE TEXT NOT NULL,
    price FLOAT NOT NULL,
    img TEXT NOT NULL,
    volume FLOAT NOT NULL,
    category_id SERIAL NOT NULL,
    subcategory_id SERIAL NOT NULL,

    abv FLOAT NOT NULL,
    aer FLOAT GENERATED ALWAYS AS (volume*abv/price) STORED,

    unit_price FLOAT GENERATED ALWAYS AS (price/volume) STORED,
    retailer retailer NOT NULL,

    checksum TEXT UNIQUE NOT NULL,

    FOREIGN KEY (category_id) REFERENCES categories (id),
    FOREIGN KEY (subcategory_id) REFERENCES subcategories (id)
);

/* Incredient references */

CREATE TABLE incredient_product_filters (
    incredient_id SERIAL NOT NULL,
    product_id SERIAL NOT NULL,

    FOREIGN KEY (incredient_id) REFERENCES drink_incredients (id),
    FOREIGN KEY (product_id) REFERENCES products (id),

    PRIMARY KEY (incredient_id, product_id)
);

CREATE TABLE user_incredients(
    user_id SERIAL NOT NULL,
    incredient_id SERIAL NOT NULL,

    FOREIGN KEY (user_id) REFERENCES users (id),
    FOREIGN KEY (incredient_id) REFERENCES drink_incredients (id),

    PRIMARY KEY (user_id, incredient_id)
);

CREATE TABLE user_favorites(
    user_id SERIAL NOT NULL,
    drink_id SERIAL NOT NULL,

    FOREIGN KEY (user_id) REFERENCES users (id),
    FOREIGN KEY (drink_id) REFERENCES drink_recipes (id),

    PRIMARY KEY (user_id, drink_id)
);

CREATE TABLE cabinets (
    id SERIAL NOT NULL PRIMARY KEY,
    owner_id SERIAL NOT NULL,
    name TEXT NOT NULL,

    access_key TEXT UNIQUE NULL DEFAULT NULL,

    FOREIGN KEY (owner_id) REFERENCES users (id)
);

CREATE TABLE shared_cabinets (
    cabinet_id SERIAL NOT NULL,
    user_id SERIAL NOT NULL,
    user_username TEXT UNIQUE NOT NULL,

    FOREIGN KEY (cabinet_id) REFERENCES cabinets (id),
    FOREIGN KEY (user_id) REFERENCES users (id),

    PRIMARY KEY (user_id, cabinet_id)
);

CREATE TABLE cabinet_products (
    cabinet_id SERIAL NOT NULL,
    product_id SERIAL NOT NULL,
    owner_id SERIAL NOT NULL,

    name TEXT NOT NULL,
    img TEXT NOT NULL,
    href TEXT NOT NULL,
    abv FLOAT NOT NULL DEFAULT 0,

    usable BOOLEAN DEFAULT true,    
    amount_ml INT NULL DEFAULT NULL,

    PRIMARY KEY (cabinet_id, product_id)
);


/* sync recipes */
CREATE OR REPLACE FUNCTION recipe_update_notify() RETURNS trigger AS $$
DECLARE
    id int;
    list varchar[];
BEGIN
    IF TG_OP = 'INSERT' OR TG_OP = 'UPDATE' THEN
        id = NEW.id;
    ELSE
        id = OLD.id;
    END IF;

    list = ARRAY(SELECT recipe_id FROM recipe_parts WHERE incredient_id = id);
    
    PERFORM pg_notify('recipe_update', json_build_object('table', TG_TABLE_NAME, 'id', id, 'list', list, 'action_type', TG_OP)::text);
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS recipe_notify_update ON drink_incredients;
CREATE TRIGGER recipe_notify_update AFTER UPDATE ON drink_incredients FOR EACH ROW EXECUTE PROCEDURE recipe_update_notify();


/* sync incredients */
CREATE OR REPLACE FUNCTION incredient_update_notify() RETURNS trigger AS $$
DECLARE
    pid int;
    list varchar[];
    list_static varchar[];
BEGIN
    IF TG_OP = 'INSERT' OR TG_OP = 'UPDATE' THEN
        pid = NEW.id;
    ELSE
        pid = OLD.id;
    END IF;

    list = ARRAY(SELECT f.incredient_id FROM products p INNER JOIN incredient_product_filters f ON f.product_id = p.id WHERE p.id = pid);
    list_static = ARRAY(SELECT d.id FROM products p INNER JOIN drink_incredients d ON d.use_static_filter AND d.static_filter = p.subcategory_id WHERE p.id = pid);
    
    PERFORM pg_notify('incredient_update', json_build_object('table', TG_TABLE_NAME, 'id', pid, 'list', list, 'list_static', list_static, 'action_type', TG_OP)::text);
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS incredient_notify_update ON products;
CREATE TRIGGER incredient_notify_update AFTER UPDATE ON products FOR EACH ROW EXECUTE PROCEDURE incredient_update_notify();