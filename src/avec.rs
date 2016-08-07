/*
NO! Premature optimization! If we did this we'd have to implement all kinds of sorting algorithms & crap!
It is predictable that we will want it tho. But it's better to suffer by lacking it than to never ship.
*/

pub struct ArrayVec<D> {
    bits: usize,
    elements: usize,
    data: Vec<Vec<D>>,
}
impl<D> ArrayVec<D> {
    pub fn new_auto() -> ArrayVec<D> {
        // We want `size_of::<D>() * (1 << bits) ≈ 16KiB`
        // So `bits ≈ log2(16KiB / size_of::<D>())
        let desired_size = (1024 * 16) as f32;
        use std::mem::size_of;
        let bits = (desired_size / size_of::<D>() as f32).log2().round() as usize;
        assert!(bits > 0);
        assert!(bits < 64);
        let actual_size = 1 << bits;
        assert!(actual_size > 1);
        assert!(actual_size <= desired_size * 2);
        ArrayVec::new(bits)
    }

    pub fn new(bits: usize) -> ArrayVec<D> {
        ArrayVec {
            bits: bits,
            elements: 0,
            data: Vec::new(),
        }
    }

    pub fn index(&self, i: usize) -> (usize, usize) {
        let low = i & ((1 << self.bits) - 1);
        let hig = i >> self.bits;
        (hig, low)
    }

    pub fn push(&mut self, d: D) {
        let size_per = 1 << self.bits;
        if self.data.is_empty() || self.data.last().len() >= size_per {
            self.data.push(Vec::with_capacity(size_per));
        }
        self.data.last().push(d);
    }

    pub fn get(&self, i: usize) -> &D {
        let (hig, low) = self.index(i);
        self.data[hig][low]
    }
    pub fn get_mut(&self, i: usize) -> &mut D {
        let (hig, low) = self.index(i);
        self.data[hig][low]
    }
    pub fn set(&self, i: usize, d: D) {
        let (hig, low) = self.index(i);
        self.data[hig][low] = d;
    }

    pub fn iter<'a>(&'a self) -> ArrayVecIter<'a, &D> {
        ArrayVecIter {
            iters: self.data.map(Vec::as_slice).collect(),
        }
    }
}

pub struct ArrayVecIter<'a, D: 'a> {
    iters: Vec<&'a [D]>,
}
impl<'a, D: 'a> Iterator for ArrayVecIter<'a, D> {
    type Item = D;
    fn next(&mut self) -> Option<D> {
        if self.iters.is_empty() { return None }
        loop {
            match self.iters[0].split_first() {
                None => self.iters.remove(0),
                Some(head, tail) => {
                    self.iters[0] = tail;
                    return Some(head);
                },
            }
        }
    }
}
