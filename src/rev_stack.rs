use core::{
    marker::PhantomData,
    mem::MaybeUninit,
    slice,
};

/* --------------------------------------------------------------------- */
/*  Reversed (grow-down) stack                                           */
/* --------------------------------------------------------------------- */

pub struct RevStackRef<'a, T> {
    base: *mut T,   // start of allocation
    cap:  usize,    // total slots
    len:  usize,    // live elements
    _p:   PhantomData<&'a mut T>,
}

unsafe impl<'a, T: Send> Send for RevStackRef<'a, T> {}
unsafe impl<'a, T: Sync> Sync for RevStackRef<'a, T> {}

impl<'a, T> RevStackRef<'a, T> {
    /* ------------- constructors ---------------- */

    pub fn from_slice(buf: &'a mut [MaybeUninit<T>]) -> Self {
        Self { base: buf.as_mut_ptr() as *mut T,
               cap:  buf.len(),
               len:  0,
               _p:   PhantomData }
    }

    pub fn new_full(buf: &'a mut [T]) -> Self {
        Self { base: buf.as_mut_ptr(),
               cap:  buf.len(),
               len:  buf.len(),
               _p:   PhantomData }
    }

    /* ------------- basic helpers --------------- */

    #[inline] pub fn len(&self)      -> usize { self.len }
    #[inline] pub fn is_empty(&self) -> bool  { self.len == 0 }
    #[inline] pub fn room_left(&self)-> usize { self.cap - self.len }
    #[inline] pub fn write_index(&self) -> usize { self.len }  // kept for compat

    /* ------------- single-element ops ---------- */

    pub fn push(&mut self, v: T) -> Result<(), T> {
        if self.len == self.cap {
            return Err(v);
        }
        self.len += 1;
        let idx = self.cap - self.len;
        unsafe { self.base.add(idx).write(v); }
        Ok(())
    }

    pub fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            return None;
        }
        let idx = self.cap - self.len;
        self.len -= 1;
        unsafe { Some(self.base.add(idx).read()) }
    }

    pub fn peek(&self) -> Option<&T> {
        if self.len == 0 {
            return None;
        }
        let idx = self.cap - self.len;
        unsafe { Some(&*self.base.add(idx)) }
    }

    /* ------------- bulk helpers ---------------- */

    /// Push an entire slice (`vals[0]` ends up deepest, `vals.last()` on top).
    pub fn push_many(&mut self, vals: &[T]) -> Result<(), ()>
    where
        T: Clone,                 // need a way to copy the values in
    {
        if self.room_left() < vals.len() {
            return Err(());
        }

        let n         = vals.len();
        let dst_start = self.cap - self.len - n;

        unsafe {
            let dst = self.base.add(dst_start);
            for (i, v) in vals.iter().rev().enumerate() {
                dst.add(i).write(v.clone());
            }
        }

        self.len += n;
        Ok(())
    }

    /// Borrow the top `n` items without popping.
    pub fn peek_many(&self, n: usize) -> Option<&[T]> {
        if n > self.len {
            return None;
        }
        let start = self.cap - self.len;
        unsafe {
            Some(slice::from_raw_parts(self.base.add(start), n))
        }
    }

    /// Pop the top `n` items **without** dropping them, returning a `&mut [T]`
    /// that lives for as long as the mutable borrow of `self`.
    ///
    /// ⚠️ The caller is now responsible for eventually dropping those `T`s.
    pub fn pop_many<'b>(&'b mut self, n: usize) -> Option<&'b mut [T]>
    where T :Copy
    {
        if n > self.len {
            return None;
        }
        let start = self.cap - self.len;
        self.len -= n;

        unsafe {
            Some(slice::from_raw_parts_mut(self.base.add(start), n))
        }
    }

    /* ------------- expose raw backing ----------- */

    pub fn into_slice(self) -> &'a mut [MaybeUninit<T>] {
        unsafe { slice::from_raw_parts_mut(self.base as *mut MaybeUninit<T>, self.cap) }
    }
}


#[cfg(test)]
mod tests {
    use crate::stack::make_storage;
use super::*;

    #[test]
    fn capacity_zero_is_safe() {
        let mut storage = make_storage::<u8, 0>();
        let mut s = RevStackRef::from_slice(&mut storage);

        assert!(s.push(1).is_err());
        assert!(s.pop().is_none());
        assert!(s.peek().is_none());
        assert!(s.is_empty());
    }

    #[test]
    fn lifo_order_single_ops() {
        let mut storage = make_storage::<&'static str, 3>();
        let mut s = RevStackRef::from_slice(&mut storage);

        s.push("A").unwrap();
        s.push("B").unwrap();
        s.push("C").unwrap();

        assert_eq!(s.peek(), Some(&"C"));
        assert_eq!(s.pop(),  Some("C"));
        assert_eq!(s.pop(),  Some("B"));
        assert_eq!(s.pop(),  Some("A"));
        assert_eq!(s.pop(),  None);
    }

    #[test]
    fn mixed_push_peek_pop_many_sequence() {
        let mut storage = make_storage::<i32, 4>();
        let mut s = RevStackRef::from_slice(&mut storage);

        s.push(10).unwrap();
        s.push(20).unwrap();

        assert!(s.push_many(&[30, 40]).is_ok());
        assert_eq!(s.len(), 4);

        let peeked = s.peek_many(2).unwrap();
        assert_eq!(peeked, &[40,30]);

        //order may seem odd here but [30 40] must be ontop
        //20 is the highest element of the previous slice so it must be the last
        let popped = s.pop_many(3).unwrap();
        assert_eq!(popped, &[40,30, 20]);

        assert_eq!(s.len(), 1);
        assert_eq!(s.peek(), Some(&10));

        // Push too many should fail
        assert!(s.push_many(&[1, 2, 3, 4, 5]).is_err());

        // Clean out final element
        assert_eq!(s.pop(), Some(10));
        assert!(s.pop().is_none());
        assert!(s.is_empty());
    }

}
    