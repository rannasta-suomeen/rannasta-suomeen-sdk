use std::{collections::HashMap, str::FromStr};


use potion::Error;
use serde_json::Value;

use super::error::TypeError;


pub type FormData = HashMap<String, Value>;

pub struct Form {
    inner: HashMap<String, Value>,
}

impl Form {
    pub fn from_data(data: FormData) -> Self {
        Self { inner: data }
    }

    pub fn get_value<T>(&self, key: &str) -> Result<T, Error>
    where
        T: TryFrom<Value>,
    {
        match self.inner.get(key) {
            Some(value) => value
                .to_owned()
                .try_into()
                .map_err(|_e| TypeError::new("Invalid type conversion").into()),
            None => Err(TypeError::new("Invalid key").into()),
        }
    }

    pub fn get_number<T>(&self, key: &str) -> Result<T, Error>
    where
        T: FromStr,
    {
        match self.inner.get(key) {
            Some(value) => match value.as_str() {
                Some(v) => v
                    .to_owned()
                    .parse()
                    .map_err(|_e| TypeError::new("Invalid type conversion").into()),
                None => Err(TypeError::new("Failed to parse value as str").into()),
            },
            None => Err(TypeError::new("Invalid key").into()),
        }
    }

    pub fn get_str(&self, key: &str) -> Result<String, TypeError> {
        match self.inner.get(key) {
            Some(value) => match value.as_str() {
                Some(v) => Ok(v.to_string()),
                None => Err(TypeError::new("Invalid key")),
            },
            None => Err(TypeError::new("Invalid key")),
        }
    }
}
