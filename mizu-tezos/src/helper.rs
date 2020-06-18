// taken from serde_with::rust::seq_display_fromstr
// (https://docs.rs/serde_with/1.4.0/serde_with/rust/seq_display_fromstr/index.html)
// with next_element::<&str> changed to next_element::<String>
pub mod seq_display_fromstr {
    use serde::{
        de::{Deserializer, Error, SeqAccess, Visitor},
        ser::{SerializeSeq, Serializer},
    };
    use std::{
        fmt::{self, Display},
        iter::{FromIterator, IntoIterator},
        marker::PhantomData,
        str::FromStr,
    };

    /// Deserialize collection T using [FromIterator] and [FromStr] for each element
    pub fn deserialize<'de, D, T, I>(deserializer: D) -> Result<T, D::Error>
    where
        D: Deserializer<'de>,
        T: FromIterator<I> + Sized,
        I: FromStr,
        I::Err: Display,
    {
        struct Helper<S>(PhantomData<S>);

        impl<'de, S> Visitor<'de> for Helper<S>
        where
            S: FromStr,
            <S as FromStr>::Err: Display,
        {
            type Value = Vec<S>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                write!(formatter, "a sequence")
            }

            fn visit_seq<A>(self, mut access: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let mut values = access
                    .size_hint()
                    .map(Self::Value::with_capacity)
                    .unwrap_or_else(Self::Value::new);

                while let Some(value) = access.next_element::<String>()? {
                    values.push(value.parse::<S>().map_err(Error::custom)?);
                }

                Ok(values)
            }
        }

        deserializer
            .deserialize_seq(Helper(PhantomData))
            .map(T::from_iter)
    }

    /// Serialize collection T using [IntoIterator] and [Display] for each element
    #[allow(dead_code)]
    pub fn serialize<S, T, I>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        for<'a> &'a T: IntoIterator<Item = &'a I>,
        I: Display,
    {
        let iter = value.into_iter();
        let (_, to) = iter.size_hint();
        let mut seq = serializer.serialize_seq(to)?;
        for item in iter {
            seq.serialize_element(&item.to_string())?;
        }
        seq.end()
    }
}
