
use std::cmp::Ordering;
use std::iter::Peekable;
/**
 * Helper for SQL-style merge joins.
 *
 *
 * Usage
 * =====
 *
 * ```
 * # extern crate v11;
 * # fn main() {
 * # use v11::joincore::*;
 * let left = vec![2, 3, 5, 7, 11];
 * let mut right = JoinCore::new(vec![1, 3, 4, 5, 7, 9].into_iter());
 * for prime in left.iter() {
 *    match right.join(prime, i32::cmp) {
 *        Join::Next => continue,
 *        Join::Stop => break,
 *        Join::Match(odd) => {
 *            println!("What an odd prime number: {}", odd);
 *        },
 *    }
 * };
 * # }
 * ```
 *
 * See Also
 * ========
 *
 * The package [joinkit](https://crates.io/crates/joinkit) may be preferable if you are
 * dealing with two streams that are both actually `Iterator`s.
 *
 * */
pub struct JoinCore<IT: Iterator> {
    right: Peekable<IT>,
}
impl<IT: Iterator> JoinCore<IT> {
    pub fn new(iter: IT) -> Self {
        JoinCore {
            right: iter.peekable(),
        }
    }

    pub fn join<'a, 'b, L: Copy + 'b, Compare>(&'a mut self, left_item: L, cmp: Compare) -> Join<IT::Item>
        where Compare: for<'c> Fn(L, &'c IT::Item) -> Ordering
    {
        while self.right.peek().is_some() {
            let right_item = self.right.next().unwrap();
            return match cmp(left_item, &right_item) {
                // left_item needs to advance
                Ordering::Less => Join::Next,
                // a good join
                Ordering::Equal => Join::Match(right_item),
                // the right side needs to advance
                Ordering::Greater => {
                    self.right.next();
                    continue;
                },
            };
        }
        Join::Stop
    }
}
impl<IT: Iterator> JoinCore<IT> where IT::Item: Ord {
    pub fn cmp(&mut self, left_item: &IT::Item) -> Join<IT::Item> {
        self.join(left_item, IT::Item::cmp)
    }
}

/// Return value for `JoinCore.join`.
pub enum Join<T> {
    /// The outer/left iterator needs to be advanced.
    Next,
    /// The inner/right iterator has no more elements.
    Stop,
    /// A matching result.
    Match(T),
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn empty_l() {
        // testing {empty_r, empty_both} makes no sense
        let l: Vec<usize> = Vec::new();
        let r: Vec<usize> = vec![1, 2, 3, 4];
        let mut jc = JoinCore::new(l.into_iter());
        for right in r.iter() {
            let right: &usize = right;
            match jc.join(right, usize::cmp) {
                Join::Stop => continue,
                _ => panic!("expected Stop"),
            }
        }
    }
    // FIXME: More tests.
}


// FIXME: Extract separate crate?
