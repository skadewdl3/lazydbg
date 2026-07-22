use serde::{Serialize, ser};
use std::fmt;

#[derive(Debug, Clone)]
pub enum ArgValue {
    Null,
    Bool(bool),
    Str(String),
    Seq(Vec<ArgValue>),
}

#[derive(Debug)]
pub struct Error(pub String);
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl std::error::Error for Error {}
impl ser::Error for Error {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Error(msg.to_string())
    }
}

/// Serializes a single field's value (bool/number/string/option/seq) into ArgValue.
pub struct ArgSerializer;

impl ser::Serializer for ArgSerializer {
    type Ok = ArgValue;
    type Error = Error;
    type SerializeSeq = SeqCollector;
    type SerializeTuple = SeqCollector;
    type SerializeTupleStruct = SeqCollector;
    type SerializeTupleVariant = ser::Impossible<ArgValue, Error>;
    type SerializeMap = ser::Impossible<ArgValue, Error>;
    type SerializeStruct = ser::Impossible<ArgValue, Error>;
    type SerializeStructVariant = ser::Impossible<ArgValue, Error>;

    fn serialize_bool(self, v: bool) -> Result<ArgValue, Error> {
        Ok(ArgValue::Bool(v))
    }
    fn serialize_i8(self, v: i8) -> Result<ArgValue, Error> {
        Ok(ArgValue::Str(v.to_string()))
    }
    fn serialize_i16(self, v: i16) -> Result<ArgValue, Error> {
        Ok(ArgValue::Str(v.to_string()))
    }
    fn serialize_i32(self, v: i32) -> Result<ArgValue, Error> {
        Ok(ArgValue::Str(v.to_string()))
    }
    fn serialize_i64(self, v: i64) -> Result<ArgValue, Error> {
        Ok(ArgValue::Str(v.to_string()))
    }
    fn serialize_u8(self, v: u8) -> Result<ArgValue, Error> {
        Ok(ArgValue::Str(v.to_string()))
    }
    fn serialize_u16(self, v: u16) -> Result<ArgValue, Error> {
        Ok(ArgValue::Str(v.to_string()))
    }
    fn serialize_u32(self, v: u32) -> Result<ArgValue, Error> {
        Ok(ArgValue::Str(v.to_string()))
    }
    fn serialize_u64(self, v: u64) -> Result<ArgValue, Error> {
        Ok(ArgValue::Str(v.to_string()))
    }
    fn serialize_f32(self, v: f32) -> Result<ArgValue, Error> {
        Ok(ArgValue::Str(v.to_string()))
    }
    fn serialize_f64(self, v: f64) -> Result<ArgValue, Error> {
        Ok(ArgValue::Str(v.to_string()))
    }
    fn serialize_char(self, v: char) -> Result<ArgValue, Error> {
        Ok(ArgValue::Str(v.to_string()))
    }
    fn serialize_str(self, v: &str) -> Result<ArgValue, Error> {
        Ok(ArgValue::Str(v.to_string()))
    }
    fn serialize_bytes(self, v: &[u8]) -> Result<ArgValue, Error> {
        Ok(ArgValue::Str(String::from_utf8_lossy(v).into_owned()))
    }
    fn serialize_none(self) -> Result<ArgValue, Error> {
        Ok(ArgValue::Null)
    }
    fn serialize_some<T: ?Sized + Serialize>(self, v: &T) -> Result<ArgValue, Error> {
        v.serialize(self)
    }
    fn serialize_unit(self) -> Result<ArgValue, Error> {
        Ok(ArgValue::Null)
    }
    fn serialize_unit_struct(self, _n: &'static str) -> Result<ArgValue, Error> {
        Ok(ArgValue::Null)
    }
    fn serialize_unit_variant(
        self,
        _n: &'static str,
        _i: u32,
        variant: &'static str,
    ) -> Result<ArgValue, Error> {
        Ok(ArgValue::Str(variant.to_string()))
    }
    fn serialize_newtype_struct<T: ?Sized + Serialize>(
        self,
        _n: &'static str,
        v: &T,
    ) -> Result<ArgValue, Error> {
        v.serialize(self)
    }
    fn serialize_newtype_variant<T: ?Sized + Serialize>(
        self,
        _n: &'static str,
        _i: u32,
        _v: &'static str,
        v: &T,
    ) -> Result<ArgValue, Error> {
        v.serialize(self)
    }
    fn serialize_seq(self, len: Option<usize>) -> Result<SeqCollector, Error> {
        Ok(SeqCollector {
            items: Vec::with_capacity(len.unwrap_or(0)),
        })
    }
    fn serialize_tuple(self, len: usize) -> Result<SeqCollector, Error> {
        Ok(SeqCollector {
            items: Vec::with_capacity(len),
        })
    }
    fn serialize_tuple_struct(self, _n: &'static str, len: usize) -> Result<SeqCollector, Error> {
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
    ) -> Result<Self::SerializeTupleVariant, Error> {
        Err(Error("tuple variants unsupported as MI args".into()))
    }
    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Error> {
        Err(Error("maps unsupported as MI args".into()))
    }
    fn serialize_struct(
        self,
        _n: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Error> {
        Err(Error("nested structs unsupported as MI args".into()))
    }
    fn serialize_struct_variant(
        self,
        _n: &'static str,
        _i: u32,
        _v: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Error> {
        Err(Error("struct variants unsupported as MI args".into()))
    }
}

pub struct SeqCollector {
    items: Vec<ArgValue>,
}
impl ser::SerializeSeq for SeqCollector {
    type Ok = ArgValue;
    type Error = Error;
    fn serialize_element<T: ?Sized + Serialize>(&mut self, v: &T) -> Result<(), Error> {
        self.items.push(v.serialize(ArgSerializer)?);
        Ok(())
    }
    fn end(self) -> Result<ArgValue, Error> {
        Ok(ArgValue::Seq(self.items))
    }
}
impl ser::SerializeTuple for SeqCollector {
    type Ok = ArgValue;
    type Error = Error;
    fn serialize_element<T: ?Sized + Serialize>(&mut self, v: &T) -> Result<(), Error> {
        self.items.push(v.serialize(ArgSerializer)?);
        Ok(())
    }
    fn end(self) -> Result<ArgValue, Error> {
        Ok(ArgValue::Seq(self.items))
    }
}
impl ser::SerializeTupleStruct for SeqCollector {
    type Ok = ArgValue;
    type Error = Error;
    fn serialize_field<T: ?Sized + Serialize>(&mut self, v: &T) -> Result<(), Error> {
        self.items.push(v.serialize(ArgSerializer)?);
        Ok(())
    }
    fn end(self) -> Result<ArgValue, Error> {
        Ok(ArgValue::Seq(self.items))
    }
}

/// Top-level: turns a `#[derive(Serialize)] struct Cmd { .. }` into ordered (field, value) pairs.
pub struct CommandSerializer;

impl ser::Serializer for CommandSerializer {
    type Ok = Vec<(String, ArgValue)>;
    type Error = Error;
    type SerializeSeq = ser::Impossible<Self::Ok, Error>;
    type SerializeTuple = ser::Impossible<Self::Ok, Error>;
    type SerializeTupleStruct = ser::Impossible<Self::Ok, Error>;
    type SerializeTupleVariant = ser::Impossible<Self::Ok, Error>;
    type SerializeMap = ser::Impossible<Self::Ok, Error>;
    type SerializeStruct = StructCollector;
    type SerializeStructVariant = ser::Impossible<Self::Ok, Error>;

    fn serialize_struct(self, _n: &'static str, len: usize) -> Result<StructCollector, Error> {
        Ok(StructCollector {
            fields: Vec::with_capacity(len),
        })
    }
    fn serialize_newtype_struct<T: ?Sized + Serialize>(
        self,
        _n: &'static str,
        v: &T,
    ) -> Result<Self::Ok, Error> {
        v.serialize(self)
    }

    fn serialize_bool(self, _v: bool) -> Result<Self::Ok, Error> {
        Err(Error("command must be a struct".into()))
    }
    fn serialize_i8(self, _v: i8) -> Result<Self::Ok, Error> {
        Err(Error("command must be a struct".into()))
    }
    fn serialize_i16(self, _v: i16) -> Result<Self::Ok, Error> {
        Err(Error("command must be a struct".into()))
    }
    fn serialize_i32(self, _v: i32) -> Result<Self::Ok, Error> {
        Err(Error("command must be a struct".into()))
    }
    fn serialize_i64(self, _v: i64) -> Result<Self::Ok, Error> {
        Err(Error("command must be a struct".into()))
    }
    fn serialize_u8(self, _v: u8) -> Result<Self::Ok, Error> {
        Err(Error("command must be a struct".into()))
    }
    fn serialize_u16(self, _v: u16) -> Result<Self::Ok, Error> {
        Err(Error("command must be a struct".into()))
    }
    fn serialize_u32(self, _v: u32) -> Result<Self::Ok, Error> {
        Err(Error("command must be a struct".into()))
    }
    fn serialize_u64(self, _v: u64) -> Result<Self::Ok, Error> {
        Err(Error("command must be a struct".into()))
    }
    fn serialize_f32(self, _v: f32) -> Result<Self::Ok, Error> {
        Err(Error("command must be a struct".into()))
    }
    fn serialize_f64(self, _v: f64) -> Result<Self::Ok, Error> {
        Err(Error("command must be a struct".into()))
    }
    fn serialize_char(self, _v: char) -> Result<Self::Ok, Error> {
        Err(Error("command must be a struct".into()))
    }
    fn serialize_str(self, _v: &str) -> Result<Self::Ok, Error> {
        Err(Error("command must be a struct".into()))
    }
    fn serialize_bytes(self, _v: &[u8]) -> Result<Self::Ok, Error> {
        Err(Error("command must be a struct".into()))
    }
    fn serialize_none(self) -> Result<Self::Ok, Error> {
        Err(Error("command must be a struct".into()))
    }
    fn serialize_some<T: ?Sized + Serialize>(self, _v: &T) -> Result<Self::Ok, Error> {
        Err(Error("command must be a struct".into()))
    }
    fn serialize_unit(self) -> Result<Self::Ok, Error> {
        Err(Error("command must be a struct".into()))
    }
    fn serialize_unit_struct(self, _n: &'static str) -> Result<Self::Ok, Error> {
        Err(Error("command must be a struct".into()))
    }
    fn serialize_unit_variant(
        self,
        _n: &'static str,
        _i: u32,
        _v: &'static str,
    ) -> Result<Self::Ok, Error> {
        Err(Error("command must be a struct".into()))
    }
    fn serialize_newtype_variant<T: ?Sized + Serialize>(
        self,
        _n: &'static str,
        _i: u32,
        _v: &'static str,
        _val: &T,
    ) -> Result<Self::Ok, Error> {
        Err(Error("command must be a struct".into()))
    }
    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Error> {
        Err(Error("command must be a struct".into()))
    }
    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Error> {
        Err(Error("command must be a struct".into()))
    }
    fn serialize_tuple_struct(
        self,
        _n: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct, Error> {
        Err(Error("command must be a struct".into()))
    }
    fn serialize_tuple_variant(
        self,
        _n: &'static str,
        _i: u32,
        _v: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Error> {
        Err(Error("command must be a struct".into()))
    }
    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Error> {
        Err(Error("command must be a struct".into()))
    }
    fn serialize_struct_variant(
        self,
        _n: &'static str,
        _i: u32,
        _v: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Error> {
        Err(Error("command must be a struct".into()))
    }
}

pub struct StructCollector {
    fields: Vec<(String, ArgValue)>,
}
impl ser::SerializeStruct for StructCollector {
    type Ok = Vec<(String, ArgValue)>;
    type Error = Error;
    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Error> {
        self.fields
            .push((key.to_string(), value.serialize(ArgSerializer)?));
        Ok(())
    }
    fn end(self) -> Result<Self::Ok, Error> {
        Ok(self.fields)
    }
}
