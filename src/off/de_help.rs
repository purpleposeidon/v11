use serde::de::{self, Deserialize, MapAccess, SeqAccess, Error};

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

    pub fn val(name: &'static str, val: T) -> Self {
        Self { name, val: Some(val) }
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

    pub fn expect<'a, 's, V>(&'s mut self, map: &'a mut V) -> Result<(), V::Error>
    where
        V: MapAccess<'de>,
        T: 'a + PartialEq,
    {
        if let Some(expect) = self.val.take() {
            match map.next_value() {
                Ok(t) => if expect == t {
                    Ok(())
                } else {
                    Err(V::Error::custom(format!("wrong value")))
                },
                Err(e) => Err(e),
            }
        } else {
            Err(V::Error::duplicate_field(self.name))
        }
    }

    pub fn expected<V>(self) -> Result<(), V::Error>
    where
        V: MapAccess<'de>,
    {
        if self.val.is_some() {
            Err(V::Error::missing_field(self.name))
        } else {
            Ok(())
        }
    }
}
