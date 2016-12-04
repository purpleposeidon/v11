
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
 * let left = vec![2, 3, 5, 7, 11];
 * let mut right = v11::joincore::InnerJoinCore::new(vec![1, 3, 4, 5, 7, 9].into_iter());
 * for prime in left.iter() {
 *    if let Some(odd) = right.join(*prime) {
 *        println!("What an odd prime number: {}", odd);
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
pub struct InnerJoinCore<I, IT: Iterator<Item=I>> {
    right: Peekable<IT>,
}
impl<I, IT: Iterator<Item=I>> InnerJoinCore<I, IT> where I: Ord {
    pub fn new(iter: IT) -> Self {
        InnerJoinCore {
            right: iter.peekable(),
        }
    }

    pub fn join(&mut self, left_item: I) -> Option<I> {
        loop {
            let cmp = if let Some(right_item) = self.right.peek() {
                left_item.cmp(right_item)
            } else {
                // It would be nice to signal an early break. Hmm!
                return None;
            };
            return match cmp {
                // left_item needs to advance
                Ordering::Less => None,
                // a good join
                Ordering::Equal => Some(left_item),
                // the right side needs to advance
                Ordering::Greater => {
                    self.right.next();
                    continue;
                },
            };
        }
    }
}

