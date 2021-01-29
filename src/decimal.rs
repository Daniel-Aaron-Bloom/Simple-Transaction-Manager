use std::fmt::{self, Formatter};
use std::num::ParseIntError;
use std::ops::{Add, AddAssign, Sub};
use std::str::{from_utf8, FromStr};

use serde::{
    de::{self, Unexpected},
    ser, Deserialize, Deserializer, Serialize, Serializer,
};

///! A lot taken from rust_decimal. Originally was thinking about just using that crate, but it seemed to have a large number of dependencies
///! which I don't have time to audit, and also it seems to be a bit overkill. Should be reasonably drop-in able though.

#[derive(Default, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Decimal {
    dollars: u64,
    cents: u16,
}

impl Decimal {
    pub fn new(dollars: u64, cents: u16) -> Self {
        let mut d = Decimal { dollars, cents };
        d += Self::zero();
        d
    }
    pub fn zero() -> Self {
        Default::default()
    }
}

impl fmt::Display for Decimal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{:04}", self.dollars, self.cents)
    }
}

impl Add for Decimal {
    type Output = Self;
    fn add(mut self, rhs: Self) -> Self {
        self += rhs;
        self
    }
}

impl AddAssign for Decimal {
    fn add_assign(&mut self, rhs: Self) {
        self.cents += rhs.cents;
        self.dollars += rhs.dollars + (self.cents / 10u16.pow(PRECISION as u32)) as u64;
        self.cents %= 10u16.pow(PRECISION as u32);
    }
}

impl Sub for Decimal {
    type Output = Result<Self, Self>;
    fn sub(self, rhs: Self) -> Result<Self, Self> {
        match (
            self.dollars.checked_sub(rhs.dollars),
            self.cents >= rhs.cents,
        ) {
            // Both parts of rhs are greater
            (None, false) => Err(Decimal {
                dollars: rhs.dollars - self.dollars,
                cents: rhs.cents - self.cents,
            }),

            // Rhs dollars are greater, lhs cents are greater
            (None, true) => Err(Decimal {
                dollars: rhs.dollars - self.dollars - 1,
                cents: 10u16.pow(PRECISION as u32) + rhs.cents - self.cents,
            }),

            // Both parts of lhs are greater or equal
            (Some(dollars), true) => Ok(Decimal {
                dollars,
                cents: self.cents - rhs.cents,
            }),

            // Dollars are equal, rhs cents are greater
            (Some(0), false) => Err(Decimal {
                dollars: 0,
                cents: rhs.cents - self.cents,
            }),

            // Lhs dollars are greater, rhs cents are greater (carry)
            (Some(dollars), false) => Ok(Decimal {
                dollars: dollars - 1,
                cents: 10u16.pow(PRECISION as u32) + self.cents - rhs.cents,
            }),
        }
    }
}

const DIGITS: usize = 20; // Above decimal
const PRECISION: usize = 4; // Below decimal

impl FromStr for Decimal {
    type Err = ParseIntError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // HACK to error if totally empty
        if s.is_empty() {
            s.parse::<u64>()?;
        }

        let (dollars, cents) = match s.find('.') {
            Some(dot) => (&s[..dot], &s[dot + 1..]),
            None => (s, &s[0..0]),
        };

        // TODO: check length of cents

        let dollars = if dollars.is_empty() {
            0
        } else {
            dollars.parse::<u64>()?
        };
        let cents = (0..PRECISION).fold(Ok(0), |total, cent_index| {
            total.and_then(|total| {
                Ok(total * 10
                    + cents
                        .get(cent_index..cent_index + 1)
                        .map_or(Ok(0), |c| u16::from_str(c))?)
            })
        })?;
        Ok(Decimal { dollars, cents })
    }
}

impl<'de> Deserialize<'de> for Decimal {
    fn deserialize<D>(deserializer: D) -> Result<Decimal, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct DecimalKey;

        impl<'de> de::Deserialize<'de> for DecimalKey {
            fn deserialize<D: de::Deserializer<'de>>(
                deserializer: D,
            ) -> Result<DecimalKey, D::Error> {
                const DECIMAL_KEY_TOKEN: &str = "$serde_json::private::Number";
                struct FieldVisitor;

                impl<'de> serde::de::Visitor<'de> for FieldVisitor {
                    type Value = ();

                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        formatter.write_str("a valid decimal field")
                    }

                    fn visit_str<E>(self, s: &str) -> Result<(), E>
                    where
                        E: serde::de::Error,
                    {
                        if s == DECIMAL_KEY_TOKEN {
                            Ok(())
                        } else {
                            Err(serde::de::Error::custom("expected field with custom name"))
                        }
                    }
                }

                deserializer.deserialize_identifier(FieldVisitor)?;
                Ok(DecimalKey)
            }
        }

        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = Decimal;

            fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
                formatter.write_str("string containing a decimal")
            }

            fn visit_str<E: de::Error>(self, value: &str) -> Result<Decimal, E> {
                Decimal::from_str(value).map_err(de::Error::custom)
            }

            fn visit_map<A: de::MapAccess<'de>>(self, mut map: A) -> Result<Decimal, A::Error> {
                if map.next_key::<DecimalKey>()?.is_none() {
                    return Err(de::Error::invalid_type(Unexpected::Map, &self));
                }
                let v: Decimal = map.next_value()?;
                Ok(v)
            }
        }

        deserializer.deserialize_str(Visitor)
    }
}

impl Serialize for Decimal {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use fmt::Write;
        struct Buffer {
            buf: [u8; DIGITS + 1 + PRECISION],
            len: usize,
        };
        impl Write for Buffer {
            fn write_str(&mut self, s: &str) -> Result<(), fmt::Error> {
                if self.len + s.len() > self.buf.len() {
                    return Err(fmt::Error);
                }
                (&mut self.buf[self.len..self.len + s.len()]).copy_from_slice(s.as_bytes());
                self.len += s.len();
                Ok(())
            }
        }
        let mut buffer = Buffer {
            buf: [0u8; DIGITS + 1 + PRECISION],
            len: 0,
        };
        write!(&mut buffer, "{}", self).map_err(ser::Error::custom)?;
        serializer.serialize_str(from_utf8(&buffer.buf[0..buffer.len]).map_err(ser::Error::custom)?)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn deserialize_basic_decimal() {
        let d: Decimal = serde_json::from_str("\"10.1234\"").unwrap();
        assert_eq!(d.dollars, 10);
        assert_eq!(d.cents, 1234);

        let d: Decimal = serde_json::from_str("\"1.2\"").unwrap();
        assert_eq!(d.dollars, 1);
        assert_eq!(d.cents, 2000);

        let d: Decimal = serde_json::from_str("\"001.002\"").unwrap();
        assert_eq!(d.dollars, 1);
        assert_eq!(d.cents, 20);

        let d: Decimal = serde_json::from_str("\".002\"").unwrap();
        assert_eq!(d.dollars, 0);
        assert_eq!(d.cents, 20);
    }

    #[test]
    fn deserialize_basic_math() {
        assert_eq!(
            Decimal::new(10, 5) + Decimal::new(1, 3),
            Decimal::new(11, 8)
        );
        assert_eq!(
            Decimal::new(10, 5000) + Decimal::new(1, 8000),
            Decimal::new(12, 3000)
        );

        assert_eq!(
            Decimal::new(10, 5000) - Decimal::new(1, 8000),
            Ok(Decimal::new(8, 7000))
        );
        assert_eq!(
            Decimal::new(1, 8000) - Decimal::new(1, 8000),
            Ok(Decimal::new(0, 0))
        );
        assert_eq!(
            Decimal::new(1, 5000) - Decimal::new(1, 8000),
            Err(Decimal::new(0, 3000))
        );
        assert_eq!(
            Decimal::new(1, 5000) - Decimal::new(5, 0),
            Err(Decimal::new(3, 5000))
        );
    }
}
