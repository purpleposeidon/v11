use std::sync::*;
use std::mem;

struct Lock<'u> {
    _lock: RwLockWriteGuard<'u, i32>,
    val: &'u mut i32,
}

fn make_lock<'u>(verse: &'u RwLock<i32>) -> Lock<'u> {
    let mut vlock = verse.write().unwrap();
    let val: &'u mut i32 = unsafe { mem::transmute(&mut *vlock) };
    Lock {
        _lock: vlock,
        val: val,
    }
}

fn main() {
    let verse = RwLock::new(42);
    let lock = make_lock(&verse);
    println!("{}", lock.val);
    *lock.val += 10;
    println!("{}", lock.val);
    let mut evil = lock.val;
    mem::drop(lock._lock);
    *evil /= 13;
    println!("{}", *evil);

    let lock = make_lock(&verse);
    *lock.val = 666;
    println!("{}", *evil);
}
