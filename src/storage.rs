use Storable;
use columns::TCol;

/// Stores data contiguously using the standard rust `Vec`.
/// This is ideal for tables that do not have rows added to them often.
#[derive(Debug)]
#[derive(RustcEncodable, RustcDecodable)]
pub struct VecCol<E: Storable> {
    data: Vec<E>,
}
impl<E: Storable> TCol for VecCol<E> {
    type Data = Vec<E>;
    type Element = E;

    fn new() -> Self { VecCol { data: Vec::new() } }
    fn data(&self) -> &Self::Data { &self.data }
    fn data_mut(&mut self) -> &mut Self::Data { &mut self.data }

    fn col_index(&self, index: usize) -> &E { &self.data[index] }
    fn col_index_mut(&mut self, index: usize) -> &mut E { &mut self.data[index] }
    unsafe fn col_index_unchecked(&self, index: usize) -> &E { self.data.get_unchecked(index) }
    unsafe fn col_index_unchecked_mut(&mut self, index: usize) -> &mut E { self.data.get_unchecked_mut(index) }
    fn len(&self) -> usize { self.data.len() }
}

/// Temporary (hopefully) stub for avec.
/// Use this for tables that may be heavily extended at run-time.
// FIXME: Implement.
pub type SegCol<E> = VecCol<E>;

type BitVec = ::bit_vec::BitVec<u64>;

/// Densely packed booleans.
#[derive(Debug, Default)]
#[derive(RustcEncodable, RustcDecodable)]
pub struct BoolCol {
    data: BitVec,
}
impl TCol for BoolCol {
    type Data = BitVec;
    type Element = bool;
    fn new() -> BoolCol {
        Default::default()
    }

    fn data(&self) -> &Self::Data { &self.data }
    fn data_mut(&mut self) -> &mut Self::Data { &mut self.data }

    fn col_index(&self, index: usize) -> &bool { &self.data[index] }
    fn col_index_mut(&mut self, index: usize) -> &mut bool { &mut self.data[index] }
    fn len(&self) -> usize { self.data.len() }
}

#[cfg(test)]
mod test {
    #[test]
    fn bool_col_unit() {
        use super::TCol;
        let mut bc = super::BoolCol::new();
        let v = &[true, false, true];
        for i in v {
            bc.data_mut().push(*i);
        }
        println!("");
        println!("Start:");
        for i in bc.data().iter() {
            println!("{}", i);
        }
        println!("Cleared:");
        bc.data_mut().clear();
        for i in bc.data().iter() {
            println!("{}", i);
        }
        println!("Really Cleared:");
        bc.data_mut().clear();
        for i in bc.data().iter() {
            println!("{}", i);
        }
        println!("Append:");
        bc.data_mut().extend(vec![true, true]);
        for i in bc.data().iter() {
            println!("{}", i);
        }
        println!("{:?}", bc);
    }
}
