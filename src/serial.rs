use serde::ser::{Serialize, Serializer, SerializeStruct};


use tracking::SelectAny;
use tables::{GenericTable, TableName, GenericColumn, BoxedSerialize};
use domain::DomainName;
pub struct TableSelection<'a> {
    pub domain: DomainName,
    pub name: TableName,
    // Lol. Okay. `SelectAny` needs to be the input argument, since this is gonna call from a
    // fallback event handler.
    // But it needs to be serialized via &SelectOwned<T>,
    // but it needs to be passed to the GenericColumn as SelectAny.
    pub selection: &'a SelectAny<'a>,
    pub serial_selection: BoxedSerialize<'a>,
    pub columns: &'a [GenericColumn],
    // Also this is *generic*. Yeah!
}
impl<'a> TableSelection<'a> {
    pub fn from(gt: &'a GenericTable, selection: &'a SelectAny<'a>) -> Self {
        TableSelection {
            domain: gt.domain,
            name: gt.name,
            selection,
            serial_selection: gt.table.serial_selection(selection),
            columns: gt.columns.as_slice(),
        }
    }
}
impl<'a> Serialize for TableSelection<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("table", 3 + self.columns.len())?;
        state.serialize_field("table:domain", &self.domain)?;
        state.serialize_field("table:name", &self.name)?;
        state.serialize_field("table:selection", &self.serial_selection)?;
        for col in self.columns {
            state.serialize_field(col.name, &(col.serializer_factory)(col, &self.selection))?;
        }
        state.end()
    }
}
