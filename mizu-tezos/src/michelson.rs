use num_bigint::BigInt;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Debug, Clone)]
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

use serde::ser::{SerializeSeq, SerializeStruct, Serializer};
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
