
use std;
use std::collections::HashMap;
use std::fmt::{self, Display, Debug};
use std::ops::Deref;
use std::sync::RwLock;

#[derive(Copy, Clone, Eq, PartialEq)]
pub struct Atom(*const usize);

//This is ok, the *const usize points to a 'static str
unsafe impl Sync for Atom {}
unsafe impl Send for Atom {}

impl Atom {
    pub fn new(s: &str) -> Self {
        INTERNED_STRINGS.write().unwrap().intern(s)
    }

    pub fn try_new(s: &str) -> Option<Self> {
        INTERNED_STRINGS.read().unwrap().get_if_interned(s)
    }

    pub fn as_str(self) -> &'static str {
        unsafe { Interner::extract_interned_string(self.0) }
    }

    pub fn get_discarded_bytes() -> usize {
        INTERNED_STRINGS.read().unwrap().allocator.get_discarded_bytes()
    }
}

impl Deref for Atom {
    type Target = str;
    fn deref(&self) -> &str {
        self.as_str()
    }
}

impl Display for Atom {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.deref())
    }
}

impl Debug for Atom {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Display::fmt(self, f)
    }
}

lazy_static! {
    static ref INTERNED_STRINGS: RwLock<Interner> = RwLock::new(Interner::new());
}

struct Interner {
    allocator: SlabAllocator<usize>,
    strings: HashMap<&'static str, Atom>
}

impl Interner {
    fn new() -> Self {
        Interner {
            allocator: SlabAllocator::new(),
            strings: HashMap::new()
        }
    }

    fn get_if_interned(&self, s: &str) -> Option<Atom> {
        self.strings.get(s).map(|a| *a)
    }

    fn intern(&mut self, s: &str) -> Atom {
        if let Some(atom) = self.strings.get(s) {
            return *atom
        }
        
        let atom = self.alloc_interned_string(s);
        self.strings.insert(atom.as_str(), atom);
        atom
    }

    fn alloc_interned_string(&mut self, s: &str) -> Atom {
        let len = s.len();
        // We allocate a buffer of usize, to have the correct alignment,
        // and we make sure to have enough room to store the string data
        let buf = self.allocator.alloc(1 + div_round_up(len, std::mem::size_of::<usize>()));
        unsafe {
            std::ptr::write(buf, len);
            std::ptr::copy_nonoverlapping(s.as_bytes().as_ptr(), buf.offset(1) as *mut u8, len);
        }
        Atom(buf)
    }

    unsafe fn extract_interned_string(ptr: *const usize) -> &'static str {
        let len = *ptr;
        let str_start = ptr.offset(1) as *const u8;
        let slice = std::slice::from_raw_parts(str_start, len);
        std::str::from_utf8_unchecked(slice)
    }
}


fn div_round_up(a: usize, b: usize) -> usize {
    a / b + if a % b == 0 { 0 } else { 1 }
}

const SLAB_ALLOC_SIZE: usize = 4096;
struct SlabAllocator<T> {
    start: *mut T, //Start of current slab
    end: *mut T, //End of current slab
    lost: usize //Total number of bytes discarded
}

//This is ok, no interior mutability
unsafe impl<T> Sync for SlabAllocator<T> {}
unsafe impl<T> Send for SlabAllocator<T> {}

impl<T> SlabAllocator<T> {
    fn new() -> Self {
        SlabAllocator {
            start: std::ptr::null_mut(),
            end: std::ptr::null_mut(),
            lost: 0
        }
    }

    fn get_discarded_bytes(&self) -> usize {
        self.lost
    }

    fn slab_free_size(&self) -> usize {
        (self.end as usize - self.start as usize) / std::mem::size_of::<T>()
    }

    fn slab_size(&self) -> usize {
        div_round_up(SLAB_ALLOC_SIZE, std::mem::size_of::<T>())
    }

    fn alloc(&mut self, len: usize) -> *mut T {
        if len >= self.slab_size() {
        // We allocate big buffers outside the slab
            let mut buf: Vec<T> = Vec::with_capacity(len);
            let start = buf.as_mut_ptr();
            std::mem::forget(buf);
            start

        } else {
            // If the slice is not big enough, we allocate a new one
            if len > self.slab_free_size() {
                self.lost += self.slab_free_size();
                let mut buf: Vec<T> = Vec::with_capacity(self.slab_size());
                unsafe {
                    self.start = buf.as_mut_ptr();
                    self.end = self.start.offset(self.slab_size() as isize);
                    std::mem::forget(buf);
                }
            }
            //We give out part of our slab
            let new_start = unsafe { self.start.offset(len as isize) };
            std::mem::replace(&mut self.start, new_start)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interning() {
        let foo = Atom::new("foo");
        assert_eq!(foo.deref(), "foo");
        assert_eq!(Atom::try_new("foo"), Some(foo));
        assert_eq!(Atom::try_new("bar"), None);
    }

    #[test]
    fn allocator() {
        fn alloc_and_test_mem(alloc: &mut SlabAllocator<usize>, size: usize) {
            let slice = unsafe { std::slice::from_raw_parts_mut(alloc.alloc(size), size) };
            for i in 0..size {
                slice[i] = i;
            }

            assert_eq!(slice.iter().cloned().sum::<usize>(), (size * (size-1)) / 2);
        }

        let mut alloc = SlabAllocator::new();
        let slab_size = alloc.slab_size();

        //Test that an empty slab get correctly used
        alloc_and_test_mem(&mut alloc, 100);
        alloc_and_test_mem(&mut alloc, 200);
        alloc_and_test_mem(&mut alloc, slab_size);
        assert_eq!(alloc.slab_free_size(), slab_size - 300);
        assert_eq!(alloc.get_discarded_bytes(), 0);

        //Test that huge vecs get allocated outsize the slab
        alloc_and_test_mem(&mut alloc, slab_size);
        assert_eq!(alloc.get_discarded_bytes(), 0);

        //Test that the slab replacement works correctly
        alloc_and_test_mem(&mut alloc, slab_size-1);
        assert_eq!(alloc.get_discarded_bytes(), slab_size - 300);
        assert_eq!(alloc.slab_free_size(), 1);

    }
}
