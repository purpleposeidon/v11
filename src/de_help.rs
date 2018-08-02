use serde::de::{self, Deserialize, MapAccess, SeqAccess};

pub fn next<'de, T, V>(
    seq: &mut V,
    name: &'static str,
) -> Result<T, V::Error>
where
    T: Deserialize<'de>,
    V: SeqAccess<'de>,
{
    match seq.next_element() {
        Ok(Some(t)) => Ok(t),
        Ok(None) => Err(de::Error::missing_field(name)),
        Err(m) => Err(m),
    }
}

/// Represents a field in a structure.
pub struct Hole<T> {
    name: &'static str,
    val: Option<T>,
}
impl<'de, T: Deserialize<'de>> Hole<T> {
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            val: None,
        }
    }

    pub fn from<V>(
        &mut self,
        seq: &mut V,
    ) -> Result<(), V::Error>
    where V: SeqAccess<'de>
    {
        match seq.next_element() {
            Ok(v) => {
                self.val = v;
                Ok(())
            },
            Err(m) => Err(m),
        }
    }

    pub fn fill<V>(
        &mut self,
        map: &mut V,
    ) -> Result<(), V::Error>
    where V: MapAccess<'de>
    {
        match map.next_value() {
            Ok(t) => {
                if self.val.is_some() {
                    Err(de::Error::duplicate_field(self.name))
                } else {
                    self.val = Some(t);
                    Ok(())
                }
            },
            Err(m) => Err(m),
        }
    }

    pub fn take<V>(
        mut self,
    ) -> Result<T, V::Error>
    where V: MapAccess<'de>
    {
        self
            .val
            .take()
            .ok_or_else(|| de::Error::missing_field(self.name))
    }
}
