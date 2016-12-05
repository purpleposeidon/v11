
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
pub struct JoinCore<I, IT: Iterator<Item=I>> {
    right: Peekable<IT>,
}
impl<I, IT: Iterator<Item=I>> JoinCore<I, IT> where I: Ord {
    pub fn new(iter: IT) -> Self {
        JoinCore {
            right: iter.peekable(),
        }
    }

    pub fn join<Compare>(&mut self, left_item: &I, cmp: Compare) -> Join<I>
        where Compare: Fn(&I, &I) -> Ordering
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

/// Return value for `JoinCore.join`.
pub enum Join<T> {
    /// The outer/left iterator needs to be advanced.
    Next,
    /// The inner/right iterator has no more elements.
    Stop,
    /// A matching result.
    Match(T),
}


// FIXME: More tests.
// FIXME: Extract separate crate.
