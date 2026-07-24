use crate::error::SerializationError;
use serde::{Serialize, ser};

#[derive(Debug, Clone)]
pub enum ArgValue {
    Null,
    Bool(bool),
    Str(String),
    Seq(Vec<ArgValue>),
}

/// Serializes a single field's value (bool/number/string/option/seq) into ArgValue.
pub struct ArgSerializer;

impl ser::Serializer for ArgSerializer {
    type Ok = ArgValue;
    type Error = SerializationError;
    type SerializeSeq = SeqCollector;
    type SerializeTuple = SeqCollector;
    type SerializeTupleStruct = SeqCollector;
    type SerializeTupleVariant = ser::Impossible<ArgValue, SerializationError>;
    type SerializeMap = ser::Impossible<ArgValue, SerializationError>;
    type SerializeStruct = ser::Impossible<ArgValue, SerializationError>;
    type SerializeStructVariant = ser::Impossible<ArgValue, SerializationError>;

    fn serialize_bool(self, v: bool) -> Result<ArgValue, SerializationError> {
        Ok(ArgValue::Bool(v))
    }
    fn serialize_i8(self, v: i8) -> Result<ArgValue, SerializationError> {
        Ok(ArgValue::Str(v.to_string()))
    }
    fn serialize_i16(self, v: i16) -> Result<ArgValue, SerializationError> {
        Ok(ArgValue::Str(v.to_string()))
    }
    fn serialize_i32(self, v: i32) -> Result<ArgValue, SerializationError> {
        Ok(ArgValue::Str(v.to_string()))
    }
    fn serialize_i64(self, v: i64) -> Result<ArgValue, SerializationError> {
        Ok(ArgValue::Str(v.to_string()))
    }
    fn serialize_u8(self, v: u8) -> Result<ArgValue, SerializationError> {
        Ok(ArgValue::Str(v.to_string()))
    }
    fn serialize_u16(self, v: u16) -> Result<ArgValue, SerializationError> {
        Ok(ArgValue::Str(v.to_string()))
    }
    fn serialize_u32(self, v: u32) -> Result<ArgValue, SerializationError> {
        Ok(ArgValue::Str(v.to_string()))
    }
    fn serialize_u64(self, v: u64) -> Result<ArgValue, SerializationError> {
        Ok(ArgValue::Str(v.to_string()))
    }
    fn serialize_f32(self, v: f32) -> Result<ArgValue, SerializationError> {
        Ok(ArgValue::Str(v.to_string()))
    }
    fn serialize_f64(self, v: f64) -> Result<ArgValue, SerializationError> {
        Ok(ArgValue::Str(v.to_string()))
    }
    fn serialize_char(self, v: char) -> Result<ArgValue, SerializationError> {
        Ok(ArgValue::Str(v.to_string()))
    }
    fn serialize_str(self, v: &str) -> Result<ArgValue, SerializationError> {
        Ok(ArgValue::Str(v.to_string()))
    }
    fn serialize_bytes(self, v: &[u8]) -> Result<ArgValue, SerializationError> {
        Ok(ArgValue::Str(String::from_utf8_lossy(v).into_owned()))
    }
    fn serialize_none(self) -> Result<ArgValue, SerializationError> {
        Ok(ArgValue::Null)
    }
    fn serialize_some<T: ?Sized + Serialize>(self, v: &T) -> Result<ArgValue, SerializationError> {
        v.serialize(self)
    }
    fn serialize_unit(self) -> Result<ArgValue, SerializationError> {
        Ok(ArgValue::Null)
    }
    fn serialize_unit_struct(self, _n: &'static str) -> Result<ArgValue, SerializationError> {
        Ok(ArgValue::Null)
    }
    fn serialize_unit_variant(
        self,
        _n: &'static str,
        _i: u32,
        variant: &'static str,
    ) -> Result<ArgValue, SerializationError> {
        Ok(ArgValue::Str(variant.to_string()))
    }
    fn serialize_newtype_struct<T: ?Sized + Serialize>(
        self,
        _n: &'static str,
        v: &T,
    ) -> Result<ArgValue, SerializationError> {
        v.serialize(self)
    }
    fn serialize_newtype_variant<T: ?Sized + Serialize>(
        self,
        _n: &'static str,
        _i: u32,
        _v: &'static str,
        v: &T,
    ) -> Result<ArgValue, SerializationError> {
        v.serialize(self)
    }
    fn serialize_seq(self, len: Option<usize>) -> Result<SeqCollector, SerializationError> {
        Ok(SeqCollector {
            items: Vec::with_capacity(len.unwrap_or(0)),
        })
    }
    fn serialize_tuple(self, len: usize) -> Result<SeqCollector, SerializationError> {
        Ok(SeqCollector {
            items: Vec::with_capacity(len),
        })
    }
    fn serialize_tuple_struct(
        self,
        _n: &'static str,
        len: usize,
    ) -> Result<SeqCollector, SerializationError> {
        Ok(SeqCollector {
            items: Vec::with_capacity(len),
        })
    }
    fn serialize_tuple_variant(
        self,
        _n: &'static str,
        _i: u32,
        _v: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, SerializationError> {
        Err(SerializationError::UnsupportedMiArg(
            "tuple variants".into(),
        ))
    }
    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, SerializationError> {
        Err(SerializationError::UnsupportedMiArg(
            "maps unsupported".into(),
        ))
    }
    fn serialize_struct(
        self,
        _n: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, SerializationError> {
        Err(SerializationError::UnsupportedMiArg(
            "nested structs".into(),
        ))
    }
    fn serialize_struct_variant(
        self,
        _n: &'static str,
        _i: u32,
        _v: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, SerializationError> {
        Err(SerializationError::UnsupportedMiArg(
            "struct variants".into(),
        ))
    }
}

pub struct SeqCollector {
    items: Vec<ArgValue>,
}
impl ser::SerializeSeq for SeqCollector {
    type Ok = ArgValue;
    type Error = SerializationError;
    fn serialize_element<T: ?Sized + Serialize>(
        &mut self,
        v: &T,
    ) -> Result<(), SerializationError> {
        self.items.push(v.serialize(ArgSerializer)?);
        Ok(())
    }
    fn end(self) -> Result<ArgValue, SerializationError> {
        Ok(ArgValue::Seq(self.items))
    }
}
impl ser::SerializeTuple for SeqCollector {
    type Ok = ArgValue;
    type Error = SerializationError;
    fn serialize_element<T: ?Sized + Serialize>(
        &mut self,
        v: &T,
    ) -> Result<(), SerializationError> {
        self.items.push(v.serialize(ArgSerializer)?);
        Ok(())
    }
    fn end(self) -> Result<ArgValue, SerializationError> {
        Ok(ArgValue::Seq(self.items))
    }
}
impl ser::SerializeTupleStruct for SeqCollector {
    type Ok = ArgValue;
    type Error = SerializationError;
    fn serialize_field<T: ?Sized + Serialize>(&mut self, v: &T) -> Result<(), SerializationError> {
        self.items.push(v.serialize(ArgSerializer)?);
        Ok(())
    }
    fn end(self) -> Result<ArgValue, SerializationError> {
        Ok(ArgValue::Seq(self.items))
    }
}

/// Top-level: turns a `#[derive(Serialize)] struct Cmd { .. }` into ordered (field, value) pairs.
pub struct CommandSerializer;

impl ser::Serializer for CommandSerializer {
    type Ok = Vec<(String, ArgValue)>;
    type Error = SerializationError;
    type SerializeSeq = ser::Impossible<Self::Ok, SerializationError>;
    type SerializeTuple = ser::Impossible<Self::Ok, SerializationError>;
    type SerializeTupleStruct = ser::Impossible<Self::Ok, SerializationError>;
    type SerializeTupleVariant = ser::Impossible<Self::Ok, SerializationError>;
    type SerializeMap = ser::Impossible<Self::Ok, SerializationError>;
    type SerializeStruct = StructCollector;
    type SerializeStructVariant = ser::Impossible<Self::Ok, SerializationError>;

    fn serialize_struct(
        self,
        _n: &'static str,
        len: usize,
    ) -> Result<StructCollector, SerializationError> {
        Ok(StructCollector {
            fields: Vec::with_capacity(len),
        })
    }
    fn serialize_newtype_struct<T: ?Sized + Serialize>(
        self,
        _n: &'static str,
        v: &T,
    ) -> Result<Self::Ok, SerializationError> {
        v.serialize(self)
    }

    // All the below serializes must return errors
    // since valid lazydbg-mi commands have to be structs
    fn serialize_bool(self, _v: bool) -> Result<Self::Ok, SerializationError> {
        Err(SerializationError::InvalidCommandFormat)
    }
    fn serialize_i8(self, _v: i8) -> Result<Self::Ok, SerializationError> {
        Err(SerializationError::InvalidCommandFormat)
    }
    fn serialize_i16(self, _v: i16) -> Result<Self::Ok, SerializationError> {
        Err(SerializationError::InvalidCommandFormat)
    }
    fn serialize_i32(self, _v: i32) -> Result<Self::Ok, SerializationError> {
        Err(SerializationError::InvalidCommandFormat)
    }
    fn serialize_i64(self, _v: i64) -> Result<Self::Ok, SerializationError> {
        Err(SerializationError::InvalidCommandFormat)
    }
    fn serialize_u8(self, _v: u8) -> Result<Self::Ok, SerializationError> {
        Err(SerializationError::InvalidCommandFormat)
    }
    fn serialize_u16(self, _v: u16) -> Result<Self::Ok, SerializationError> {
        Err(SerializationError::InvalidCommandFormat)
    }
    fn serialize_u32(self, _v: u32) -> Result<Self::Ok, SerializationError> {
        Err(SerializationError::InvalidCommandFormat)
    }
    fn serialize_u64(self, _v: u64) -> Result<Self::Ok, SerializationError> {
        Err(SerializationError::InvalidCommandFormat)
    }
    fn serialize_f32(self, _v: f32) -> Result<Self::Ok, SerializationError> {
        Err(SerializationError::InvalidCommandFormat)
    }
    fn serialize_f64(self, _v: f64) -> Result<Self::Ok, SerializationError> {
        Err(SerializationError::InvalidCommandFormat)
    }
    fn serialize_char(self, _v: char) -> Result<Self::Ok, SerializationError> {
        Err(SerializationError::InvalidCommandFormat)
    }
    fn serialize_str(self, _v: &str) -> Result<Self::Ok, SerializationError> {
        Err(SerializationError::InvalidCommandFormat)
    }
    fn serialize_bytes(self, _v: &[u8]) -> Result<Self::Ok, SerializationError> {
        Err(SerializationError::InvalidCommandFormat)
    }
    fn serialize_none(self) -> Result<Self::Ok, SerializationError> {
        Err(SerializationError::InvalidCommandFormat)
    }
    fn serialize_some<T: ?Sized + Serialize>(self, _v: &T) -> Result<Self::Ok, SerializationError> {
        Err(SerializationError::InvalidCommandFormat)
    }
    fn serialize_unit(self) -> Result<Self::Ok, SerializationError> {
        Err(SerializationError::InvalidCommandFormat)
    }
    fn serialize_unit_struct(self, _n: &'static str) -> Result<Self::Ok, SerializationError> {
        Err(SerializationError::InvalidCommandFormat)
    }
    fn serialize_unit_variant(
        self,
        _n: &'static str,
        _i: u32,
        _v: &'static str,
    ) -> Result<Self::Ok, SerializationError> {
        Err(SerializationError::InvalidCommandFormat)
    }
    fn serialize_newtype_variant<T: ?Sized + Serialize>(
        self,
        _n: &'static str,
        _i: u32,
        _v: &'static str,
        _val: &T,
    ) -> Result<Self::Ok, SerializationError> {
        Err(SerializationError::InvalidCommandFormat)
    }
    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, SerializationError> {
        Err(SerializationError::InvalidCommandFormat)
    }
    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, SerializationError> {
        Err(SerializationError::InvalidCommandFormat)
    }
    fn serialize_tuple_struct(
        self,
        _n: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct, SerializationError> {
        Err(SerializationError::InvalidCommandFormat)
    }
    fn serialize_tuple_variant(
        self,
        _n: &'static str,
        _i: u32,
        _v: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, SerializationError> {
        Err(SerializationError::InvalidCommandFormat)
    }
    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, SerializationError> {
        Err(SerializationError::InvalidCommandFormat)
    }
    fn serialize_struct_variant(
        self,
        _n: &'static str,
        _i: u32,
        _v: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, SerializationError> {
        Err(SerializationError::InvalidCommandFormat)
    }
}

pub struct StructCollector {
    fields: Vec<(String, ArgValue)>,
}
impl ser::SerializeStruct for StructCollector {
    type Ok = Vec<(String, ArgValue)>;
    type Error = SerializationError;
    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), SerializationError> {
        self.fields
            .push((key.to_string(), value.serialize(ArgSerializer)?));
        Ok(())
    }
    fn end(self) -> Result<Self::Ok, SerializationError> {
        Ok(self.fields)
    }
}
