use tracking::SelectAny;
use tables::{GenericTable, TableName, GenericColumn, BoxedSerialize};
use domain::DomainName;

use serde::ser::{Serialize, Serializer, SerializeMap};
pub struct TableSelectionSer<'a> {
    pub domain: DomainName,
    pub name: TableName,
    pub schema_version: u32,
    // Lol. Okay. `SelectAny` needs to be the input argument, since this is gonna call from a
    // fallback event handler.
    // But it needs to be serialized via &SelectOwned<T>,
    // but it needs to be passed to the GenericColumn as SelectAny.
    pub selection: &'a SelectAny<'a>,
    pub serial_selection: BoxedSerialize<'a>,
    pub columns: &'a [GenericColumn],
    // Also this is *generic*. Yeah!
}
impl<'a> TableSelectionSer<'a> {
    pub fn from(gt: &'a GenericTable, selection: &'a SelectAny<'a>) -> Self {
        TableSelectionSer {
            domain: gt.domain,
            name: gt.name,
            schema_version: gt.schema_version,
            selection,
            serial_selection: gt.table.serial_selection(selection),
            columns: gt.columns.as_slice(),
        }
    }
}
impl<'a> Serialize for TableSelectionSer<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_map(Some(3 + self.columns.len()))?;
        state.serialize_entry(&"table:fmt", &0u32)?;
        state.serialize_entry(&"table:domain", &self.domain)?;
        state.serialize_entry(&"table:name", &self.name)?;
        state.serialize_entry(&"table:schema_version", &self.schema_version)?;
        state.serialize_entry(&"table:selection", &self.serial_selection)?;
        for col in self.columns {
            state.serialize_entry(&col.name, &(col.serializer_factory)(col, &self.selection))?;
        }
        state.end()
    }
}


// The Serialize impl on Col is "implemented" in $table.
use serde::de::{Deserialize, Deserializer, Visitor, SeqAccess};
use columns::{Col, TCol};
use tables::GetTableName;
use std::fmt;

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


use Universe;
pub trait TableDeserial: GetTableName {
    /// Serialization can be done through `TTable`.
    fn deserialize_rows<'de, D: Deserializer<'de>>(universe: &Universe, deserializer: D) -> Result<(), D::Error>;
}
