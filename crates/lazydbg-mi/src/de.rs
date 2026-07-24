use crate::Value;
use crate::error::DeserializationError;
use serde::de;
use serde::de::{
    IntoDeserializer,
    value::{MapDeserializer, SeqDeserializer},
};

impl<'de> IntoDeserializer<'de, DeserializationError> for Value {
    type Deserializer = Value;
    fn into_deserializer(self) -> Value {
        self
    }
}

macro_rules! deserialize_num {
    ($($method:ident => $visit:ident: $ty:ty),* $(,)?) => {
        $(fn $method<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, DeserializationError> {
            match self {
                Value::Str(s) => visitor.$visit(s.parse::<$ty>().map_err(|_| DeserializationError::ParseInt)?),
                other => Err(DeserializationError::ExpectedString(format!("{other:?}"))),
            }
        })*
    };
}

/// MI encodes everything as strings/tuples/lists; this bridges that into any typed struct.
impl<'de> de::Deserializer<'de> for Value {
    type Error = DeserializationError;

    fn deserialize_any<V: de::Visitor<'de>>(
        self,
        visitor: V,
    ) -> Result<V::Value, DeserializationError> {
        match self {
            Value::Str(s) => visitor.visit_string(s),
            Value::List(items) => visitor.visit_seq(SeqDeserializer::new(items.into_iter())),
            Value::Tuple(map) => visitor.visit_map(MapDeserializer::new(map.into_iter())),
        }
    }

    fn deserialize_option<V: de::Visitor<'de>>(
        self,
        visitor: V,
    ) -> Result<V::Value, DeserializationError> {
        visitor.visit_some(self) // missing keys are handled by serde's Option default on structs
    }

    fn deserialize_bool<V: de::Visitor<'de>>(
        self,
        visitor: V,
    ) -> Result<V::Value, DeserializationError> {
        match self {
            Value::Str(s) => match s.as_str() {
                "y" | "true" | "1" => visitor.visit_bool(true),
                "n" | "false" | "0" => visitor.visit_bool(false),
                other => Err(DeserializationError::ExpectedBool(format!("{other:?}"))),
            },
            other => Err(DeserializationError::ExpectedString(format!("{other:?}"))),
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
