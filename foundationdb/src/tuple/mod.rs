// Copyright 2018 foundationdb-rs developers, https://github.com/bluejekyll/foundationdb-rs/graphs/contributors
// Copyright 2013-2018 Apple, Inc and the FoundationDB project authors.
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

//! Tuple Key type like that of other FoundationDB libraries

pub mod item;

use std::{self, io::Write, string::FromUtf8Error};

#[derive(Debug, Fail)]
pub enum Error {
    #[fail(display = "Unexpected end of file")]
    EOF,
    #[fail(display = "Invalid type: {}", value)]
    InvalidType { value: u8 },
    #[fail(display = "Invalid data")]
    InvalidData,
    #[fail(display = "UTF8 conversion error")]
    FromUtf8Error(FromUtf8Error),
}

type Result<T> = std::result::Result<T, Error>;

impl From<FromUtf8Error> for Error {
    fn from(error: FromUtf8Error) -> Self {
        Error::FromUtf8Error(error)
    }
}

pub trait Encode {
    fn encode<W: Write>(&self, _w: &mut W) -> std::io::Result<()>;
    fn encode_to_vec(&self) -> Vec<u8> {
        let mut v = Vec::new();
        self.encode(&mut v)
            .expect("tuple encoding should never fail");
        v
    }
}

pub trait Decode: Sized {
    fn decode(buf: &[u8]) -> Result<Self>;
}

macro_rules! tuple_impls {
    ($($len:expr => ($($n:tt $name:ident)+))+) => {
        $(
            impl<$($name),+> Encode for ($($name,)+)
            where
                $($name: item::Encode + Default,)+
            {
                #[allow(non_snake_case, unused_assignments, deprecated)]
                fn encode<W: Write>(&self, w: &mut W) -> std::io::Result<()> {
                    $(
                        self.$n.encode(w)?;
                    )*
                    Ok(())
                }
            }

            impl<$($name),+> Decode for ($($name,)+)
            where
                $($name: item::Decode + Default,)+
            {
                #[allow(non_snake_case, unused_assignments, deprecated)]
                fn decode(buf: &[u8]) -> Result<Self> {
                    let mut buf = buf;
                    let mut out: Self = Default::default();
                    $(
                        let (v0, offset0) = $name::decode(buf)?;
                        out.$n = v0;
                        buf = &buf[offset0..];
                    )*

                    if !buf.is_empty() {
                        return Err(Error::InvalidData);
                    }

                    Ok(out)
                }
            }
        )+
    }
}

tuple_impls! {
    1 => (0 T0)
    2 => (0 T0 1 T1)
    3 => (0 T0 1 T1 2 T2)
    4 => (0 T0 1 T1 2 T2 3 T3)
    5 => (0 T0 1 T1 2 T2 3 T3 4 T4)
    6 => (0 T0 1 T1 2 T2 3 T3 4 T4 5 T5)
    7 => (0 T0 1 T1 2 T2 3 T3 4 T4 5 T5 6 T6)
    8 => (0 T0 1 T1 2 T2 3 T3 4 T4 5 T5 6 T6 7 T7)
    9 => (0 T0 1 T1 2 T2 3 T3 4 T4 5 T5 6 T6 7 T7 8 T8)
    10 => (0 T0 1 T1 2 T2 3 T3 4 T4 5 T5 6 T6 7 T7 8 T8 9 T9)
    11 => (0 T0 1 T1 2 T2 3 T3 4 T4 5 T5 6 T6 7 T7 8 T8 9 T9 10 T10)
    12 => (0 T0 1 T1 2 T2 3 T3 4 T4 5 T5 6 T6 7 T7 8 T8 9 T9 10 T10 11 T11)
}

#[derive(Clone, Debug, PartialEq)]
pub struct Value(pub Vec<item::Value>);

impl Encode for Value {
    fn encode<W: Write>(&self, w: &mut W) -> std::io::Result<()> {
        use self::item::Encode;
        for item in self.0.iter() {
            item.encode(w)?;
        }
        Ok(())
    }
}

impl Decode for Value {
    fn decode(buf: &[u8]) -> Result<Self> {
        let mut data = buf;
        let mut v = Vec::new();
        while !data.is_empty() {
            let (s, offset): (item::Value, _) = item::Decode::decode(data)?;
            v.push(s);
            data = &data[offset..];
        }
        Ok(Value(v))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_malformed_int() {
        assert!(Value::decode(&[21, 0]).is_ok());
        assert!(Value::decode(&[22, 0]).is_err());
        assert!(Value::decode(&[22, 0, 0]).is_ok());

        assert!(Value::decode(&[19, 0]).is_ok());
        assert!(Value::decode(&[18, 0]).is_err());
        assert!(Value::decode(&[18, 0, 0]).is_ok());
    }

    #[test]
    fn test_decode_tuple() {
        assert_eq!((0, ()), Decode::decode(&[20, 0]).unwrap());
    }

    #[test]
    fn test_decode_tuple_ty() {
        let data: &[u8] = &[2, 104, 101, 108, 108, 111, 0, 1, 119, 111, 114, 108, 100, 0];

        let (v1, v2): (String, Vec<u8>) = Decode::decode(data).unwrap();
        assert_eq!(v1, "hello");
        assert_eq!(v2, b"world");
    }

    #[test]
    fn test_encode_tuple_ty() {
        let tup = (String::from("hello"), b"world".to_vec());

        assert_eq!(
            &[2, 104, 101, 108, 108, 111, 0, 1, 119, 111, 114, 108, 100, 0],
            Encode::encode_to_vec(&tup).as_slice()
        );
    }
}
