use crate::parsers::mi::Value;
use serde::de::{
    self, IntoDeserializer,
    value::{MapDeserializer, SeqDeserializer},
};
use std::fmt;

#[derive(Debug)]
pub struct Error(pub String);
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl std::error::Error for Error {}
impl de::Error for Error {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Error(msg.to_string())
    }
}

impl<'de> IntoDeserializer<'de, Error> for Value {
    type Deserializer = Value;
    fn into_deserializer(self) -> Value {
        self
    }
}

macro_rules! deserialize_num {
    ($($method:ident => $visit:ident: $ty:ty),* $(,)?) => {
        $(fn $method<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
            match self {
                Value::Str(s) => visitor.$visit(s.parse::<$ty>().map_err(|e| Error(e.to_string()))?),
                other => Err(Error(format!("expected numeric string, got {other:?}"))),
            }
        })*
    };
}

/// MI encodes everything as strings/tuples/lists; this bridges that into any typed struct.
impl<'de> de::Deserializer<'de> for Value {
    type Error = Error;

    fn deserialize_any<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        match self {
            Value::Str(s) => visitor.visit_string(s),
            Value::List(items) => visitor.visit_seq(SeqDeserializer::new(items.into_iter())),
            Value::Tuple(map) => visitor.visit_map(MapDeserializer::new(map.into_iter())),
        }
    }

    fn deserialize_option<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        visitor.visit_some(self) // missing keys are handled by serde's Option default on structs
    }

    fn deserialize_bool<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        match self {
            Value::Str(s) => match s.as_str() {
                "y" | "true" | "1" => visitor.visit_bool(true),
                "n" | "false" | "0" => visitor.visit_bool(false),
                other => Err(Error(format!("expected bool-like string, got {other:?}"))),
            },
            other => Err(Error(format!("expected string for bool, got {other:?}"))),
        }
    }

    deserialize_num!(
        deserialize_i8 => visit_i8: i8, deserialize_i16 => visit_i16: i16,
        deserialize_i32 => visit_i32: i32, deserialize_i64 => visit_i64: i64,
        deserialize_u8 => visit_u8: u8, deserialize_u16 => visit_u16: u16,
        deserialize_u32 => visit_u32: u32, deserialize_u64 => visit_u64: u64,
        deserialize_f32 => visit_f32: f32, deserialize_f64 => visit_f64: f64,
    );

    serde::forward_to_deserialize_any! {
        char str string bytes byte_buf unit unit_struct newtype_struct
        seq tuple tuple_struct map struct enum identifier ignored_any i128 u128
    }
}
