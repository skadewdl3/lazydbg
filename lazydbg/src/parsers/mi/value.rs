use serde::de::DeserializeOwned;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Str(String),
    Tuple(HashMap<String, Value>),
    List(Vec<Value>),
}

impl Value {
    pub fn parse_into<T: DeserializeOwned>(self) -> Result<T, crate::parsers::mi::de::Error> {
        T::deserialize(self)
    }
}

/// Lets `Value` itself be used as a command's `Reply` type when the MI
/// result shape isn't concretely documented (many "N.A." commands below) —
/// deserialize into `Value` and pattern-match at the call site.
impl<'de> serde::de::Deserialize<'de> for Value {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        struct ValueVisitor;
        impl<'de> serde::de::Visitor<'de> for ValueVisitor {
            type Value = Value;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(f, "an MI value (string, tuple, or list)")
            }
            fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Value, E> {
                Ok(Value::Str(v.to_string()))
            }
            fn visit_string<E: serde::de::Error>(self, v: String) -> Result<Value, E> {
                Ok(Value::Str(v))
            }
            fn visit_seq<A: serde::de::SeqAccess<'de>>(
                self,
                mut seq: A,
            ) -> Result<Value, A::Error> {
                let mut items = Vec::new();
                while let Some(v) = seq.next_element::<Value>()? {
                    items.push(v);
                }
                Ok(Value::List(items))
            }
            fn visit_map<A: serde::de::MapAccess<'de>>(
                self,
                mut map: A,
            ) -> Result<Value, A::Error> {
                let mut m = HashMap::new();
                while let Some((k, v)) = map.next_entry::<String, Value>()? {
                    m.insert(k, v);
                }
                Ok(Value::Tuple(m))
            }
        }
        deserializer.deserialize_any(ValueVisitor)
    }
}

#[derive(Debug, Clone)]
pub struct EmptyReply {}
impl<'de> serde::de::Deserialize<'de> for EmptyReply {
    fn deserialize<D>(_d: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        Ok(EmptyReply {})
    }
}
