use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum ParamValue {
    Float(f64),
    Integer(i64),
    String(String),
}

impl ParamValue {
    pub fn as_int(&self) -> Option<i64> {
        if let Self::Integer(x) = self {
            Some(*x)
        } else {
            None
        }
    }

    pub fn as_float(&self) -> Option<f64> {
        if let Self::Float(x) = self {
            Some(*x)
        } else {
            None
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        if let Self::String(x) = self {
            Some(x.as_str())
        } else {
            None
        }
    }
}

impl From<f64> for ParamValue {
    fn from(value: f64) -> Self {
        Self::Float(value)
    }
}

impl From<i64> for ParamValue {
    fn from(value: i64) -> Self {
        Self::Integer(value)
    }
}

impl From<&str> for ParamValue {
    fn from(value: &str) -> Self {
        Self::String(value.to_string())
    }
}

impl From<String> for ParamValue {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

impl std::fmt::Display for ParamValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Float(x) => write!(f, "{:?}", x),
            Self::Integer(x) => f.write_str(&x.to_string()),
            Self::String(x) => f.write_str(x),
        }
    }
}
