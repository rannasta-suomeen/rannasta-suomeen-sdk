use std::{fmt::Debug, future::Future};

use potion::HtmlError;
use redis::{aio::MultiplexedConnection, AsyncCommands, FromRedisValue, ToRedisArgs};
use redis_macros::{FromRedisValue, ToRedisArgs};
use serde::{Deserialize, Serialize};

use crate::error::CacheError;

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
            CacheKeyType::Custom(_) => self._value.to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum CacheKeyType {
    Recipe,
    Ingredient,
    Custom(String),
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
            CacheKeyType::Ingredient => CacheLifetime::BindIncredientCache,
            CacheKeyType::Custom(value) => CacheLifetime::Custom(value),
        }
    }
}

// Cache - wrappers

#[derive(Serialize, Deserialize, Clone)]
pub enum CacheLifetime {
    Infinite,
    Custom(String),
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
            CacheLifetime::Custom(value) => Ok(Some(value.to_owned())),
        }
    }

    pub async fn validate_cache_bind(
        &self,
        bind: &Option<String>,
        lifetime: Self,
        cache: &mut MultiplexedConnection,
    ) -> Result<bool, potion::Error> {
        match self {
            CacheLifetime::Custom(value) => match lifetime {
                CacheLifetime::Custom(_value) => Ok(value == &_value),
                _ => {
                    log::error!("Found conflicting bindings");
                    Err(HtmlError::InternalServerError.new("Conflicting cache bindings"))
                }
            },
            _ => Ok(bind == &self.get_cache_bind(cache).await?),
        }
    }
}

#[derive(Serialize, serde::Deserialize, FromRedisValue, ToRedisArgs, Clone)]
pub struct RedisValue<T: serde::Serialize + Send + Sync + Clone> {
    pub value: T,
    _lifetime: CacheLifetime,
    _bind: Option<String>,
}

impl<T: serde::Serialize + Send + Sync + Clone + for<'a> Deserialize<'a>> RedisValue<T> {
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

    async fn validate<K: ToString + Serialize>(
        &self,
        key: CacheKey<K>,
        cache: &mut MultiplexedConnection,
    ) -> Result<bool, potion::Error> {
        self._lifetime
            .validate_cache_bind(&&self._bind, key.into(), cache)
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
        let value = get_cache_value::<String, RedisValue<T>>((&key).into(), cache)
            .await
            .unwrap_or_else(|_| {
                let mut c = cache.clone();
                let k = key.to_string();
                tokio::spawn(async move {
                    log::error!("> Failed to serialize cached value. Deleting {}", &k);
                    if let Err(e) = delete_cache_value(k, &mut c).await {
                        log::error!("> Failed to delete cached value! {e}");
                    }
                });
                None
            });
        // * Cannot use .map(|| {...}) due to async closures
        let value = match value {
            Some(value) => {
                log::trace!("> Found {:?}", key.to_string());
                match value.validate(key.to_owned(), cache).await? {
                    true => Some(value),
                    false => {
                        log::trace!("> Invalidated {}", key.to_string());
                        None
                    }
                }
            }
            None => None,
        };

        match value {
            Some(value) => Ok(Some(value)),
            None => {
                log::trace!("> Fetching {:?}", key.to_string());
                match callback().await? {
                    Some(value) => {
                        let lifetime: CacheLifetime = key.to_owned().into();
                        let value = RedisValue::new(value, lifetime, cache).await?;

                        match set_cache_value::<String, RedisValue<T>>(
                            (&key).into(),
                            value.clone(),
                            cache,
                        )
                        .await
                        {
                            Ok(_) => {}
                            Err(e) => {
                                log::error!("{e:?}");
                            }
                        }

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
        let value = get_cache_value::<String, RedisValue<T>>((&key).into(), cache)
            .await
            .unwrap_or_else(|_| {
                let mut c = cache.clone();
                let k = key.to_string();
                tokio::spawn(async move {
                    log::error!("> Failed to serialize cached value. Deleting {}", &k);
                    if let Err(e) = delete_cache_value(k, &mut c).await {
                        log::error!("> Failed to delete cached value! {e}");
                    }
                });
                None
            });
        // * Cannot use .map(|| {...}) due to async closures
        let value = match value {
            Some(value) => {
                log::trace!("> Found {:?}", key.to_string());
                match value.validate(key.to_owned().into(), cache).await? {
                    true => Some(value),
                    false => {
                        log::trace!("> Invalidated {:?}", key.to_string());
                        None
                    }
                }
            }
            None => None,
        };

        match value {
            Some(value) => Ok(value),
            None => {
                log::trace!("> Fetching {:?}", key._value.to_string());
                let value = callback().await?;
                let lifetime: CacheLifetime = key.to_owned().into();
                let value = RedisValue::new(value, lifetime, cache).await?;

                set_cache_value::<String, RedisValue<T>>((&key).into(), value.clone(), cache)
                    .await?;

                Ok(value)
            }
        }
    }

    pub async fn get_or_list<'a, F, Fut, K>(
        key: CacheKey<K>,
        cache: &mut MultiplexedConnection,
        callback: F,
    ) -> Result<RedisValue<Vec<T>>, potion::Error>
    where
        Vec<T>: serde::Serialize + Send + Sync,
        K: ToString + Serialize + Clone + Send + Sync,
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = Result<Vec<T>, potion::Error>> + Send + 'a,
    {
        let value = get_cache_value::<String, RedisValue<Vec<T>>>((&key).into(), cache)
            .await
            .unwrap_or_else(|_| {
                let mut c = cache.clone();
                let k = key.to_string();
                tokio::spawn(async move {
                    log::error!("> Failed to serialize cached value. Deleting {}", &k);
                    if let Err(e) = delete_cache_value(k, &mut c).await {
                        log::error!("> Failed to delete cached value! {e}");
                    }
                });
                None
            });
        // * Cannot use .map(|| {...}) due to async closures
        let value = match value {
            Some(value) => {
                log::trace!("> Found {:?}", key.to_string());
                match value.validate(key.to_owned().into(), cache).await? {
                    true => Some(value),
                    false => {
                        log::trace!("> Invalidated {:?}", key.to_string());
                        None
                    }
                }
            }
            None => None,
        };

        match value {
            Some(value) => Ok(value),
            None => {
                log::trace!("> Fetching {:?}", key._value.to_string());
                let value = callback().await?;
                let lifetime: CacheLifetime = key.to_owned().into();
                let value = RedisValue::new(value, lifetime, cache).await?;

                set_cache_value::<String, RedisValue<Vec<T>>>((&key).into(), value.clone(), cache)
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

pub async fn delete_cache_value<K: ToRedisArgs + Send + Sync>(
    key: K,
    cache: &mut MultiplexedConnection,
) -> Result<(), potion::Error> {
    let _: () = cache
        .del(key)
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
