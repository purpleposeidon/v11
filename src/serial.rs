use serde::ser::{Serialize, Serializer, SerializeSeq};
use serde::de::{Deserialize, Deserializer, Visitor, SeqAccess};

use columns::{Col, TCol};
use tables::GetTableName;
use std::fmt;

impl<C, T> Serialize for Col<C, T>
where
    C: TCol,
    T: GetTableName,
    C::Element: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let inner = self.inner();
        let mut seq = serializer.serialize_seq(Some(inner.len()))?;
        for i in 0..inner.len() {
            let e = unsafe { inner.unchecked_index(i) };
            seq.serialize_element(e)?;
        }
        seq.end()
    }
}
impl<'de, C, T> Deserialize<'de> for Col<C, T>
where
    C: TCol,
    T: GetTableName,
    C::Element: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let col = Col::new();
        deserializer.deserialize_seq(col)
    }
}
impl<'de, C, T> Visitor<'de> for Col<C, T>
where
    C: TCol,
    T: GetTableName,
    C::Element: Deserialize<'de>,
{
    type Value = Col<C, T>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "a column")
    }

    fn visit_seq<A>(mut self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        {
            let inner = self.inner_mut();
            if let Some(hint) = seq.size_hint() {
                inner.reserve(hint);
            }
            while let Some(e) = seq.next_element()? {
                inner.push(e);
            }
        }
        Ok(self)
    }
}
