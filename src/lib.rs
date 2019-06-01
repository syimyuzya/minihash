use std::num::{NonZeroU32, Wrapping};
use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering::SeqCst;

#[derive(Debug, Default)]
struct Entry {
    key: AtomicU32,
    value: AtomicU32,
}

#[derive(Debug)]
pub struct MiniHash {
    entries: Box<[Entry]>,
}

impl MiniHash {
    pub fn new(capacity: usize) -> Self {
        let mut vec = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            vec.push(Entry::default());
        }
        let entries = vec.into_boxed_slice();
        MiniHash { entries }
    }
    pub fn capacity(&self) -> usize {
        self.entries.len()
    }

    pub fn set(&self, key: NonZeroU32, value: NonZeroU32) {
        let key = key.get();
        let capacity = self.capacity();
        let mut i = simple_hash(key) as usize % capacity;
        for _ in 0..capacity {
            let entry = &self.entries[i];
            let entry_key = entry.key.load(SeqCst);
            if entry_key != key {
                if entry_key != 0 {
                    i = (i + 1) % capacity;
                    continue;
                }
                let prev_key = entry.key.compare_and_swap(0, key, SeqCst);
                if prev_key != 0 && prev_key != key {
                    i = (i + 1) % capacity;
                    continue;
                }
            }
            entry.value.store(value.get(), SeqCst);
            return;
        }
        panic!("array is full"); // XXX
    }
    pub fn get(&self, key: NonZeroU32) -> Option<NonZeroU32> {
        let key = key.get();
        let capacity = self.capacity();
        let mut i = simple_hash(key) as usize % capacity;
        for _ in 0..capacity {
            let entry = &self.entries[i];
            let entry_key = entry.key.load(SeqCst);
            if entry_key == key {
                return NonZeroU32::new(entry.value.load(SeqCst));
            }
            if entry_key == 0 {
                return None;
            }
            i = (i + 1) % capacity;
        }
        None
    }

    pub fn len(&self) -> usize {
        for (i, entry) in self.entries.iter().enumerate() {
            if entry.key.load(SeqCst) == 0 || entry.value.load(SeqCst) == 0 {
                return i;
            }
        }
        self.entries.len()
    }
    pub fn is_empty(&self) -> bool {
        self.len() > 0
    }
}

fn simple_hash(x: u32) -> u32 {
    let mut x = Wrapping(x);
    x ^= x >> 16;
    x *= Wrapping(0x85eb_ca6b);
    x ^= x >> 13;
    x *= Wrapping(0xc2b2_ae35);
    x ^= x >> 16;
    x.0
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::{Arc, Barrier};
    use std::thread;

    fn test_minihash(num: u32) {
        let arr = Arc::new(MiniHash::new(num as usize * 2));
        let barrier = Arc::new(Barrier::new(2));
        let a = Arc::clone(&arr);
        let b = Arc::clone(&barrier);
        let handle1 = thread::spawn(move || {
            for i in 0..num {
                let k = i * 2 + 2;
                a.set(NonZeroU32::new(k).unwrap(), NonZeroU32::new(k + 1).unwrap());
            }
            b.wait();
            for k in 1..=(num * 2) {
                assert_eq!(
                    k + 1,
                    a.get(NonZeroU32::new(k).unwrap())
                        .map(NonZeroU32::get)
                        .unwrap_or(0),
                    "{:?}",
                    *a,
                );
            }
        });
        let a = Arc::clone(&arr);
        let b = Arc::clone(&barrier);
        let handle2 = thread::spawn(move || {
            for i in 0..num {
                let k = (num - 1 - i) * 2 + 1;
                a.set(NonZeroU32::new(k).unwrap(), NonZeroU32::new(k + 1).unwrap());
            }
            b.wait();
            for k in (1..=(num * 2)).rev() {
                assert_eq!(
                    k + 1,
                    a.get(NonZeroU32::new(k).unwrap())
                        .map(NonZeroU32::get)
                        .unwrap_or(0),
                    "{:?}",
                    *a,
                );
            }
        });
        handle1.join().unwrap();
        handle2.join().unwrap();
    }

    #[test]
    fn it_works() {
        test_minihash(24000);
    }
}
