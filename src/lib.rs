use std::num::NonZeroU32;
use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering::SeqCst;

#[derive(Debug, Default)]
struct Entry {
    key: AtomicU32,
    value: AtomicU32,
}

#[derive(Debug)]
pub struct ArrayOfItems {
    entries: Box<[Entry]>,
}

impl ArrayOfItems {
    pub fn new(capacity: usize) -> Self {
        let mut vec = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            vec.push(Entry::default());
        }
        let entries = vec.into_boxed_slice();
        ArrayOfItems { entries }
    }
    pub fn capacity(&self) -> usize {
        self.entries.len()
    }

    pub fn set(&self, key: NonZeroU32, value: NonZeroU32) {
        let key = key.get();
        for entry in self.entries.iter() {
            let entry_key = entry.key.load(SeqCst);
            if entry_key != key {
                if entry_key != 0 {
                    continue;
                }
                let prev_key = entry.key.compare_and_swap(0, key, SeqCst);
                if prev_key != 0 && prev_key != key {
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
        for entry in self.entries.iter() {
            let entry_key = entry.key.load(SeqCst);
            if entry_key == key {
                return NonZeroU32::new(entry.value.load(SeqCst));
            }
            if entry_key == 0 {
                return None;
            }
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

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::{Arc, Barrier};
    use std::thread;

    fn test_array_of_items(num: u32) {
        let arr = Arc::new(ArrayOfItems::new(num as usize * 2));
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
        test_array_of_items(4000);
    }
}
