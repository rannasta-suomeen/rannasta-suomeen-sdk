use std::{fmt::Debug, future::Future};

use redis::{aio::MultiplexedConnection, AsyncCommands, FromRedisValue, ToRedisArgs};
use redis_macros::{FromRedisValue, ToRedisArgs};
use serde::{Deserialize, Serialize};

use crate::error::CacheError;

/// General trait for structs that can be cached
pub trait Cacheable: serde::Serialize + Send + Sync + Debug + Clone {}

// Caching - keys

#[derive(Serialize, ToRedisArgs, FromRedisValue, Clone, Debug)]
pub struct CacheKey<T: ToString + Serialize> {
    _value: T,
    _type: CacheKeyType,
}

impl<T: ToString + Serialize> CacheKey<T> {
    pub fn from(r#type: CacheKeyType, key: T) -> Self {
        Self {
            _value: key,
            _type: r#type,
        }
    }

    pub fn to_string(&self) -> String {
        self.into()
    }
}

impl<T: ToString + Serialize> Into<String> for &CacheKey<T> {
    fn into(self) -> String {
        match self._type {
            CacheKeyType::Recipe => format!("recipe-{}", self._value.to_string()),
            CacheKeyType::Ingredient => format!("incredient-{}", self._value.to_string()),
            CacheKeyType::RecipeParts => format!("recipe-parts-{}", self._value.to_string()),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum CacheKeyType {
    Recipe,
    RecipeParts,
    Ingredient,
}

impl CacheKeyType {
    pub fn new<T: ToString + Serialize>(self, key: T) -> CacheKey<T> {
        CacheKey::from(self, key)
    }
}

impl<T: ToString + Serialize> Into<CacheLifetime> for CacheKey<T> {
    fn into(self) -> CacheLifetime {
        match self._type {
            CacheKeyType::Recipe => CacheLifetime::BindRecipeCache,
            CacheKeyType::RecipeParts => CacheLifetime::BindRecipeCache,
            CacheKeyType::Ingredient => CacheLifetime::BindIncredientCache,
        }
    }
}

// Cache - wrappers

#[derive(Serialize, Deserialize, Clone)]
pub enum CacheLifetime {
    Infinite,
    BindGlobalCache,
    BindRecipeCache,
    BindIncredientCache,
}

impl CacheLifetime {
    pub async fn get_cache_bind(
        &self,
        cache: &mut MultiplexedConnection,
    ) -> Result<Option<String>, potion::Error> {
        match self {
            CacheLifetime::Infinite => Ok(None),
            CacheLifetime::BindGlobalCache => {
                get_cache_value::<&str, String>("global-cache-key", cache).await
            }
            CacheLifetime::BindRecipeCache => {
                get_cache_value::<&str, String>("recipe-cache-key", cache).await
            }
            CacheLifetime::BindIncredientCache => {
                get_cache_value::<&str, String>("ingredient-cache-key", cache).await
            }
        }
    }

    pub async fn validate_cache_bind(
        &self,
        bind: &Option<String>,
        cache: &mut MultiplexedConnection,
    ) -> Result<bool, potion::Error> {
        Ok(bind == &self.get_cache_bind(cache).await?)
    }
}

#[derive(Serialize, serde::Deserialize, FromRedisValue, ToRedisArgs, Clone)]
pub struct RedisValue<T: Cacheable> {
    pub value: T,
    _lifetime: CacheLifetime,
    _bind: Option<String>,
}

impl<T: Cacheable + for<'a> Deserialize<'a>> RedisValue<T> {
    async fn new(
        value: T,
        lifetime: CacheLifetime,
        cache: &mut MultiplexedConnection,
    ) -> Result<Self, potion::Error> {
        let bind = lifetime.get_cache_bind(cache).await?;

        Ok(Self {
            value,
            _lifetime: lifetime,
            _bind: bind,
        })
    }

    async fn validate(&self, cache: &mut MultiplexedConnection) -> Result<bool, potion::Error> {
        self._lifetime
            .validate_cache_bind(&&self._bind, cache)
            .await
    }

    // I'm next to god in Rust...
    pub async fn get_or_optional<'a, F, Fut, K>(
        key: CacheKey<K>,
        cache: &mut MultiplexedConnection,
        callback: F,
    ) -> Result<Option<RedisValue<T>>, potion::Error>
    where
        K: ToString + Serialize + Clone + Send + Sync,
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = Result<Option<T>, potion::Error>> + Send + 'a,
    {
        let value = get_cache_value::<String, RedisValue<T>>((&key).into(), cache).await?;
        // * Cannot use .map(|| {...}) due to async closures
        let value = match value {
            Some(value) => {
                log::info!("> Found {:?}", key._value.to_string());
                match value.validate(cache).await? {
                    true => Some(value),
                    false => {
                        log::warn!("> Invalidated {:?}", key._value.to_string());
                        None
                    }
                }
            }
            None => None,
        };

        match value {
            Some(value) => Ok(Some(value)),
            None => {
                log::info!("> Fetching {:?}", key.to_string());
                match callback().await? {
                    Some(value) => {
                        let lifetime: CacheLifetime = key.to_owned().into();
                        let value = RedisValue::new(value, lifetime, cache).await?;

                        set_cache_value::<String, RedisValue<T>>(
                            (&key).into(),
                            value.clone(),
                            cache,
                        )
                        .await?;

                        Ok(Some(value))
                    }
                    None => Ok(None),
                }
            }
        }
    }

    pub async fn get_or<'a, F, Fut, K>(
        key: CacheKey<K>,
        cache: &mut MultiplexedConnection,
        callback: F,
    ) -> Result<RedisValue<T>, potion::Error>
    where
        K: ToString + Serialize + Clone + Send + Sync,
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = Result<T, potion::Error>> + Send + 'a,
    {
        let value = get_cache_value::<String, RedisValue<T>>((&key).into(), cache).await?;
        // * Cannot use .map(|| {...}) due to async closures
        let value = match value {
            Some(value) => {
                log::info!("> Found {:?}", key._value.to_string());
                match value.validate(cache).await? {
                    true => Some(value),
                    false => {
                        log::warn!("> Invalidated {:?}", key._value.to_string());
                        None
                    }
                }
            }
            None => None,
        };

        match value {
            Some(value) => Ok(value),
            None => {
                log::info!("> Fetching {:?}", key._value.to_string());
                let value = callback().await?;
                let lifetime: CacheLifetime = key.to_owned().into();
                let value = RedisValue::new(value, lifetime, cache).await?;

                set_cache_value::<String, RedisValue<T>>((&key).into(), value.clone(), cache)
                    .await?;

                Ok(value)
            }
        }
    }
}

// Cache - raw handlers

pub async fn set_cache_value<K: ToRedisArgs + Send + Sync, V: ToRedisArgs + Send + Sync>(
    key: K,
    value: V,
    cache: &mut MultiplexedConnection,
) -> Result<(), potion::Error> {
    let _: () = cache
        .set(key, value)
        .await
        .map_err(|e| CacheError::from(e).into())?;

    Ok(())
}

pub async fn get_cache_value<K: ToRedisArgs + Send + Sync, V: FromRedisValue>(
    key: K,
    cache: &mut MultiplexedConnection,
) -> Result<Option<V>, potion::Error> {
    let value: Option<V> = cache
        .get(key)
        .await
        .map_err(|e| CacheError::from(e).into())?;

    Ok(value)
}
