use num_bigint::BigInt;
use serde::de;
use serde::de::{Deserializer, MapAccess, SeqAccess, Visitor};
use serde::ser::{SerializeSeq, SerializeStruct, Serializer};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone)]
pub enum Expr {
    Int(BigInt),
    String(String),
    Bytes(Vec<u8>),
    List(Vec<Expr>),
    Prim { prim: String, args: Vec<Expr> },
}

impl Expr {
    pub fn left(arg: Expr) -> Expr {
        Expr::Prim {
            prim: "Left".into(),
            args: vec![arg],
        }
    }
    pub fn right(arg: Expr) -> Expr {
        Expr::Prim {
            prim: "Right".into(),
            args: vec![arg],
        }
    }
    pub fn nat(value: BigInt) -> Expr {
        Expr::Prim {
            prim: "nat".into(),
            args: vec![Expr::Int(value)],
        }
    }
    pub fn pair(left: Expr, right: Expr) -> Expr {
        Expr::Prim {
            prim: "Pair".into(),
            args: vec![left, right],
        }
    }
    pub fn some(arg: Option<Expr>) -> Expr {
        match arg {
            Some(arg) => Expr::Prim {
                prim: "Some".into(),
                args: vec![arg],
            },
            None => Expr::Prim {
                prim: "None".into(),
                args: Vec::new(),
            },
        }
    }
}

impl Serialize for Expr {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Expr::Int(n) => {
                let mut state = serializer.serialize_struct("Expr", 1)?;
                state.serialize_field("int", &n.to_string())?;
                state.end()
            }
            // TODO: this is actually non-compliant if s is not UTF-8,
            // check $unistring definition
            Expr::String(s) => {
                let mut state = serializer.serialize_struct("Expr", 1)?;
                state.serialize_field("string", &s)?;
                state.end()
            }
            Expr::Bytes(b) => {
                let mut state = serializer.serialize_struct("Expr", 1)?;
                state.serialize_field("bytes", &hex::encode(&b))?;
                state.end()
            }
            Expr::List(exprs) => {
                let mut seq = serializer.serialize_seq(Some(exprs.len()))?;
                for expr in exprs.iter() {
                    seq.serialize_element(expr)?;
                }
                seq.end()
            }
            Expr::Prim { prim, args } => {
                if args.is_empty() {
                    let mut state = serializer.serialize_struct("Expr", 1)?;
                    state.serialize_field("prim", prim)?;
                    state.end()
                } else {
                    let mut state = serializer.serialize_struct("Expr", 2)?;
                    state.serialize_field("prim", prim)?;
                    state.serialize_field("args", args)?;
                    state.end()
                }
            }
        }
    }
}

struct ExprVisitor;

impl<'de> Visitor<'de> for ExprVisitor {
    type Value = Expr;
    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a Michelson expression")
    }

    fn visit_map<A>(self, mut value: A) -> Result<Expr, A::Error>
    where
        A: MapAccess<'de>,
    {
        let key: String = value
            .next_key()?
            .ok_or_else(|| de::Error::custom("no key found"))?;
        match &key[..] {
            "int" => value.next_value().map(Expr::Int),
            "string" => value.next_value().map(Expr::String),
            "bytes" => {
                let hex_string: String = value.next_value()?;
                let bytes = hex::decode(hex_string).map_err(|e| match e {
                    hex::FromHexError::InvalidHexCharacter { c, index } => {
                        de::Error::custom(format!("invalid character '{}' at index {}", c, index))
                    }
                    hex::FromHexError::OddLength => de::Error::custom("odd length"),
                    hex::FromHexError::InvalidStringLength => {
                        panic!("internal error (hex::FromHexError::InvalidStringLength)")
                    }
                })?;
                Ok(Expr::Bytes(bytes))
            }
            "args" => {
                let args = value.next_value()?;
                let key: String = value
                    .next_key()?
                    .ok_or_else(|| de::Error::custom("expecting \"prim\" key"))?;
                if key == "prim" {
                    let prim = value.next_value()?;
                    Ok(Expr::Prim { prim, args })
                } else {
                    Err(de::Error::custom(format!(
                        "expecting \"prim\" but found \"{}\"",
                        key
                    )))
                }
            }
            "prim" => {
                let prim = value.next_value()?;
                let key: String = value
                    .next_key()?
                    .ok_or_else(|| de::Error::custom("expecting \"args\" key"))?;
                if key == "args" {
                    let args = value.next_value()?;
                    Ok(Expr::Prim { prim, args })
                } else {
                    Err(de::Error::custom(format!(
                        "expecting \"args\" but found \"{}\"",
                        key
                    )))
                }
            }
            _ => Err(de::Error::custom(format!("unexpected key \"{}\"", key))),
        }
    }

    fn visit_seq<A>(self, mut value: A) -> Result<Expr, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut exprs = Vec::new();
        while let Some(expr) = value.next_element()? {
            exprs.push(expr);
        }
        Ok(Expr::List(exprs))
    }
}

impl<'de> Deserialize<'de> for Expr {
    fn deserialize<D>(deserializer: D) -> Result<Expr, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(ExprVisitor)
    }
}
