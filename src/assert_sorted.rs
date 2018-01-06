/// This is a 'speedbump type' wrapping an iterator over sorted items.
///
/// # Usage
///
/// ```
/// # use v11::tables::AssertSorted;
///
/// fn print_sorted<IT: Iterator<Item=isize>, I: Into<AssertSorted<IT>>>(iter: I) {
///     for item in iter.into() {
///         println!("{}", item);
///     }
/// }
///
/// print_sorted(AssertSorted(1..6));
/// print_sorted(vec![1, 5, 2, 3, 4]);
/// ```
pub struct AssertSorted<T: Iterator>(pub T);

impl<T> From<Vec<T>> for AssertSorted<::std::vec::IntoIter<T>>
where T: Ord
{
    fn from(mut vec: Vec<T>) -> Self {
        vec.sort();
        AssertSorted(vec.into_iter())
    }
}

use std::iter::IntoIterator;
impl<T> IntoIterator for AssertSorted<T>
where T: Iterator
{
    type Item = T::Item;
    type IntoIter = T;
    fn into_iter(self) -> Self::IntoIter {
        self.0
    }
}
