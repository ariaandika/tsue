use serde::{
    Deserializer,
    de::{self, Error, IntoDeserializer, MapAccess, SeqAccess},
};
use std::marker::PhantomData;

pub struct Sequence<'a> {
    path: std::str::Split<'a, char>,
    target: std::str::Split<'static, char>,
    val: Option<&'a str>,
}

impl<'de> MapAccess<'de> for Sequence<'de> {
    type Error = de::value::Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: de::DeserializeSeed<'de>,
    {
        loop {
            match (self.path.next(), self.target.next()) {
                (Some(val), Some(p2)) if p2.starts_with(':') => {
                    let key = p2.trim_start_matches(':');
                    self.val = Some(val);
                    return seed.deserialize(key.into_deserializer()).map(Some)
                },

                (Some(_), Some("*")) => continue,
                (Some(_), Some(_)) => continue,

                (None, None) => return Ok(None),
                _ => return Err(Self::Error::custom(
                    "path should have the same fragments, route matching is flawed, this is a bug"
                ))
            }
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: de::DeserializeSeed<'de>,
    {
        seed.deserialize(
            self.val
                .take()
                .expect("`next_value` called before `next_key`")
                .into_deserializer(),
        )
    }
}

impl<'de> SeqAccess<'de> for Sequence<'de> {
    type Error = de::value::Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: de::DeserializeSeed<'de>
    {
        loop {
            match (self.path.next(), self.target.next()) {
                (Some(_), Some(p2)) if p2.starts_with(':') => {
                    let key = p2.trim_start_matches(':');
                    return seed.deserialize(key.into_deserializer()).map(Some)
                },

                (Some(_), Some("*")) => continue,
                (Some(_), Some(_)) => continue,

                (None, None) => return Ok(None),
                _ => return Err(Self::Error::custom(
                    "path should have the same fragments, route matching is flawed, this is a bug"
                ))
            }
        }
    }
}

// ===== Extractor =====

pub struct Extractor<'a,D> {
    path: &'a str,
    target: &'static str,
    _p: PhantomData<D>
}

impl<'a, D> Extractor<'a, D> {
    pub fn new(path: &'a str, target: &'static str) -> Self {
        Self { path, target, _p: PhantomData }
    }

    fn sequence(&self) -> Sequence<'a> {
        let path = self.path.split('/');
        let target = self.target.split('/');
        Sequence { path, target, val: None }
    }

    fn single(&self, field: &'static str) -> Result<&str, de::value::Error> {
        let mut path = self.path.split('/');
        let mut target = self.target.split('/');
        let val = loop {
            match (path.next(), target.next()) {
                (Some(_), Some(p2)) if p2.starts_with(':') => {
                    break p2.trim_start_matches(':')
                },

                (Some(_), Some("*")) => continue,
                (Some(_), Some(_)) => continue,

                (None, None) => return Err(de::value::Error::missing_field(field)),
                _ => return Err(de::value::Error::custom(
                    "path should have the same fragments, route matching is flawed, this is a bug"
                )),
            }
        };

        Ok(val)
    }
}

macro_rules! forward {
    ($ty:ty, $name:ident) => {
        fn $name<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where
            V: serde::de::Visitor<'de>,
        {
            self.single(stringify!($ty))
                .map(<_>::into_deserializer)
                .and_then(|e| e.$name(visitor))
        }
    };
}

impl<'de, D> Deserializer<'de> for Extractor<'de, D> {
    type Error = de::value::Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>
    {
        visitor.visit_map(self.sequence())
    }

    forward!(bool, deserialize_bool);
    forward!(i8,  deserialize_i8);
    forward!(i16, deserialize_i16);
    forward!(i32, deserialize_i32);
    forward!(i64, deserialize_i64);
    forward!(u8,  deserialize_u8);
    forward!(u16, deserialize_u16);
    forward!(u32, deserialize_u32);
    forward!(u64, deserialize_u64);
    forward!(f32, deserialize_f32);
    forward!(f64, deserialize_f64);
    forward!(char, deserialize_char);
    forward!(str, deserialize_str);
    forward!(String, deserialize_string);
    forward!(Bytes, deserialize_bytes);
    forward!(ByteBuf, deserialize_byte_buf);
    forward!(Option, deserialize_option);
    forward!(Unit, deserialize_unit);

    fn deserialize_unit_struct<V>(
        self,
        _: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>
    {
        self.deserialize_unit(visitor)
    }

    fn deserialize_newtype_struct<V>(
        self,
        _: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>
    {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>
    {
        visitor.visit_seq(self.sequence())
    }

    fn deserialize_tuple<V>(self, _: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>
    {
        visitor.visit_seq(self.sequence())
    }

    fn deserialize_tuple_struct<V>(
        self,
        _: &'static str,
        _: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>
    {
        visitor.visit_seq(self.sequence())
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_map(self.sequence())
    }

    fn deserialize_struct<V>(
        self,
        _: &'static str,
        _: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>
    {
        visitor.visit_map(self.sequence())
    }

    fn deserialize_enum<V>(
        self,
        _: &'static str,
        _: &'static [&'static str],
        _: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>
    {
        Err(de::value::Error::custom("`enum` are not supported"))
    }

    forward!(Identifier, deserialize_identifier);

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        self.single(stringify!(Identifier))
            .map(<_>::into_deserializer)
            .and_then(|e| e.deserialize_ignored_any(visitor))
    }
}

