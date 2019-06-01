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
        // XXX change to optimized version
        for entry in self.entries.iter() {
            let prev = entry.key.compare_and_swap(0, key, SeqCst);
            if prev == 0 || prev == key {
                entry.value.store(value.get(), SeqCst);
                return;
            }
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

    use std::sync::Arc;
    use std::thread;

    #[test]
    fn it_works() {
        let arr = Arc::new(ArrayOfItems::new(8192));
        let a1 = Arc::clone(&arr);
        let handle1 = thread::spawn(move || {
            for i in 0..4000 {
                let k = i * 2 + 2;
                a1.set(NonZeroU32::new(k).unwrap(), NonZeroU32::new(k + 1).unwrap());
            }
            for k in 1..=8000 {
                assert_eq!(a1.get(NonZeroU32::new(k).unwrap()).unwrap().get(), k + 1);
            }
        });
        let a2 = Arc::clone(&arr);
        let handle2 = thread::spawn(move || {
            for i in 0..4000 {
                let k = (4000 - 1 - i) * 2 + 1;
                a2.set(NonZeroU32::new(k).unwrap(), NonZeroU32::new(k + 1).unwrap());
            }
            for k in 1..=8000 {
                assert_eq!(a2.get(NonZeroU32::new(k).unwrap()).unwrap().get(), k + 1);
            }
        });
        handle1.join().unwrap();
        handle2.join().unwrap();
    }
}
