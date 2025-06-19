use serde::{
    de::{
        self, Error as _, IntoDeserializer, MapAccess,
        value::{Error, MapDeserializer, SeqDeserializer},
    },
    forward_to_deserialize_any,
};

// ===== Parameter Iterator =====

/// Iterator that yield extracted route parameters.
struct ParamIter<'a> {
    path: std::str::Split<'a, char>,
    target: std::str::Split<'static, char>,
}

impl<'a> Iterator for ParamIter<'a> {
    type Item = (Param<'a>, Param<'a>);

    fn next(&mut self) -> Option<Self::Item> {
        match (self.path.next(), self.target.next()) {
            (Some(val), Some(p2)) if p2.starts_with(':') => {
                let key = p2.trim_start_matches(':');
                Some((Param { value: key, name: key }, Param { value: val, name: key }))
            },

            (Some(_), Some("*")) => self.next(),
            (Some(_), Some(_)) => self.next(),

            (None, None) => None,
            _ => None,
        }
    }
}

// ===== Parameter Value Iterator =====

/// Iterator that yield extracted route parameters.
struct ParamValueIter<'a> {
    path: std::str::Split<'a, char>,
    target: std::str::Split<'static, char>,
}

impl<'a> Iterator for ParamValueIter<'a> {
    type Item = Param<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        match (self.path.next(), self.target.next()) {
            (Some(val), Some(p2)) if p2.starts_with(':') => {
                let key = p2.trim_start_matches(':');
                Some(Param { value: val, name: key })
            },

            (Some(_), Some("*")) => self.next(),
            (Some(_), Some(_)) => self.next(),

            (None, None) => None,
            _ => None,
        }
    }
}

// ===== Serde Deserializer =====

/// Iterator that yield extracted route parameter.
pub struct Deserializer<'de> {
    // inner: MapDeserializer<'de, ParamIter<'de>, Error>,
    path: &'de str, target: &'static str
}

impl<'de> Deserializer<'de> {
    pub fn new(path: &'de str, target: &'static str) -> Self {
        Self { path, target }
    }

    fn map(self) -> MapDeserializer<'de, ParamIter<'de>, Error> {
        MapDeserializer::new(ParamIter {
            path: self.path.split('/'),
            target: self.target.split('/'),
        })
    }

    fn seq(self) -> SeqDeserializer<ParamValueIter<'de>, Error> {
        SeqDeserializer::new(ParamValueIter {
            path: self.path.split('/'),
            target: self.target.split('/'),
        })
    }

    fn deserialize_single<T>(self) -> Result<T, Error>
    where
        T: de::Deserialize<'de>,
    {
        let mut map = self.map();
        match map.next_entry::<&'de str, T>()? {
            Some((_, ok)) => Ok(ok),
            None => Err(Error::missing_field("first element"),)
        }
    }
}

impl<'de> de::Deserializer<'de> for Deserializer<'de> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>
    {
        self.deserialize_map(visitor)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_map(self.map())
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_seq(self.seq())
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>
    {
        visitor.visit_unit()
    }

    fn deserialize_tuple<V>(self, _: usize, visitor: V) -> Result<V::Value, Self::Error>
        where
            V: de::Visitor<'de>
    {
        visitor.visit_seq(self.seq())
    }

    forward_to_deserialize_single! {
        bool => deserialize_bool => visit_bool,
        u8 => deserialize_u8 => visit_u8,
        u16 => deserialize_u16 => visit_u16,
        u32 => deserialize_u32 => visit_u32,
        u64 => deserialize_u64 => visit_u64,
        i8 => deserialize_i8 => visit_i8,
        i16 => deserialize_i16 => visit_i16,
        i32 => deserialize_i32 => visit_i32,
        i64 => deserialize_i64 => visit_i64,
        f32 => deserialize_f32 => visit_f32,
        f64 => deserialize_f64 => visit_f64,
    }

    forward_to_deserialize_any! {
        char
        str
        string
        option
        bytes
        byte_buf
        unit_struct
        newtype_struct
        tuple_struct
        struct
        identifier
        enum
        ignored_any
    }
}

// ===== Parameter Value Deserializer =====

pub struct Param<'a> {
    value: &'a str,
    name: &'static str,
}

impl<'de> IntoDeserializer<'de> for Param<'de> {
    type Deserializer = Self;

    fn into_deserializer(self) -> Self::Deserializer {
        self
    }
}

impl<'de> de::Deserializer<'de> for Param<'de> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>
    {
        visitor.visit_borrowed_str(self.value)
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>
    {
        visitor.visit_some(self)
    }

    fn deserialize_enum<V>(
        self,
        _: &'static str,
        _: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>
    {
        visitor.visit_enum(ValueEnumAccess(self.value))
    }

    fn deserialize_newtype_struct<V>(
        self,
        _: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>
    {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        match self.value {
            "y" | "1" | "true" => visitor.visit_bool(true),
            "n" | "0" | "false" => visitor.visit_bool(false),
            _ => Err(Error::custom(format!("expected `{}` to be boolean",self.name)))
        }
    }

    forward_to_deserialize_any! {
        char
        str
        string
        unit
        bytes
        byte_buf
        unit_struct
        tuple_struct
        struct
        identifier
        tuple
        ignored_any
        seq
        map
    }

    forward_parsed_value! {
        u8 => deserialize_u8,
        u16 => deserialize_u16,
        u32 => deserialize_u32,
        u64 => deserialize_u64,
        i8 => deserialize_i8,
        i16 => deserialize_i16,
        i32 => deserialize_i32,
        i64 => deserialize_i64,
        f32 => deserialize_f32,
        f64 => deserialize_f64,
    }
}

// ===== Accessors =====

struct ValueEnumAccess<'de>(&'de str);

impl<'de> de::EnumAccess<'de> for ValueEnumAccess<'de> {
    type Error = Error;
    type Variant = UnitOnlyVariantAccess;

    fn variant_seed<V>(
        self,
        seed: V,
    ) -> Result<(V::Value, Self::Variant), Self::Error>
    where
        V: de::DeserializeSeed<'de>,
    {
        let variant = seed.deserialize(self.0.into_deserializer())?;
        Ok((variant, UnitOnlyVariantAccess))
    }
}

struct UnitOnlyVariantAccess;

impl<'de> de::VariantAccess<'de> for UnitOnlyVariantAccess {
    type Error = Error;

    fn unit_variant(self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn newtype_variant_seed<T>(self, _seed: T) -> Result<T::Value, Self::Error>
    where
        T: de::DeserializeSeed<'de>,
    {
        Err(Error::custom("expected unit variant"))
    }

    fn tuple_variant<V>(
        self,
        _len: usize,
        _visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        Err(Error::custom("expected unit variant"))
    }

    fn struct_variant<V>(
        self,
        _fields: &'static [&'static str],
        _visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        Err(Error::custom("expected unit variant"))
    }
}

// ===== Error =====

impl crate::response::IntoResponse for serde::de::value::Error {
    fn into_response(self) -> crate::response::Response {
        (http::StatusCode::BAD_REQUEST, self.to_string()).into_response()
    }
}

// ===== Macros =====

macro_rules! forward_parsed_value {
    ($($ty:ident => $method:ident,)*) => {
        $(
            fn $method<V>(self, visitor: V) -> Result<V::Value, Self::Error>
                where V: de::Visitor<'de>
            {
                match self.value.parse::<$ty>() {
                    Ok(val) => val.into_deserializer().$method(visitor),
                    _ => Err(Error::custom(format!("expected `{}` to be integer",self.name)))
                }
            }
        )*
    }
}

macro_rules! forward_to_deserialize_single {
    ($($ty:ident => $method:ident => $visit:ident,)*) => {
        $(
            fn $method<V>(self, visitor: V) -> Result<V::Value, Self::Error>
                where V: de::Visitor<'de>
            {
                match self.deserialize_single() {
                    Ok(ok) => visitor.$visit(ok),
                    Err(err) => Err(err)
                }
            }
        )*
    }
}

use {forward_parsed_value, forward_to_deserialize_single};
