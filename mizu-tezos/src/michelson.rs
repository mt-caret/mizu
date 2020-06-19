use num_bigint::BigInt;
use serde::de;
use serde::de::{Deserializer, MapAccess, SeqAccess, Visitor};
use serde::ser::{SerializeSeq, SerializeStruct, Serializer};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Prim {
    pub prim: String,
    pub args: Vec<Expr>,
}

impl Prim {
    pub fn new(prim: &str, args: &[Expr]) -> Prim {
        Prim {
            prim: prim.into(),
            args: args.iter().cloned().collect(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Expr {
    Int(BigInt),
    String(String),
    Bytes(Vec<u8>),
    List(Vec<Expr>),
    Prim(Prim),
}

impl Expr {
    pub fn prim(prim: &str, args: &[Expr]) -> Expr {
        Expr::Prim(Prim::new(prim, args))
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
            Expr::Prim(prim) => prim.serialize(serializer),
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
        let key = value
            .next_key()?
            .ok_or_else(|| de::Error::custom("no key found"))?;
        match key {
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
            // TODO: is it possible that we may get "args" first?
            "prim" => {
                let prim = value.next_value()?;
                let key: String = value
                    .next_key()?
                    .ok_or_else(|| de::Error::custom("expecting \"args\" key"))?;
                if key == "args" {
                    let args: Vec<Expr> = value.next_value()?;
                    Ok(Expr::prim(prim, &args))
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
