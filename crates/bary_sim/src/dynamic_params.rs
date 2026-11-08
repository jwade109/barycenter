use bary_core::prelude::ParamValue;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct DynamicParams {
    inner: BTreeMap<String, ParamValue>,
}

impl DynamicParams {
    pub fn new() -> Self {
        Self {
            inner: BTreeMap::new(),
        }
    }

    pub fn inner(&self) -> &BTreeMap<String, ParamValue> {
        &self.inner
    }

    pub fn insert(&mut self, key: String, value: ParamValue) {
        self.inner.insert(key, value);
    }

    pub fn get_int(&mut self, key: &str, default: i64) -> i64 {
        self.inner
            .get(key)
            .and_then(|p| p.as_int())
            .unwrap_or(default)
    }

    pub fn get_float(&mut self, key: &str, default: f64) -> f64 {
        self.inner
            .get(key)
            .and_then(|p| p.as_float())
            .unwrap_or(default)
    }

    pub fn get_str<'a>(&'a mut self, key: &str, default: &'a str) -> &'a str {
        self.inner
            .get(key)
            .and_then(|p| p.as_str())
            .unwrap_or(default)
    }
}

impl From<BTreeMap<String, ParamValue>> for DynamicParams {
    fn from(value: BTreeMap<String, ParamValue>) -> Self {
        Self { inner: value }
    }
}
