#![no_std]

use core::slice;
use core::mem::MaybeUninit;
use core::marker::PhantomData;

pub fn make_storage<T,const SIZE:usize>() ->[MaybeUninit<T>;SIZE]{
    [const { MaybeUninit::uninit() };SIZE]
}

pub struct StackRef<'a, T> {
    base: *mut T,
    head: *mut T,
    end: *mut T,

    _phantom:PhantomData<&'a T>
}

impl<'a, T> StackRef<'a, T>{
    pub fn from_raw(mem:&'a mut [MaybeUninit<T>]) -> Self{
        let base = mem.as_mut_ptr() as _;
        Self{
            base,
            head: base,
            end: unsafe {base.add(mem.len())},

            _phantom:PhantomData,
        }
    }

    pub fn push(&mut self,v:T) -> Result<(),T> {
        if self.head == self.end {
            return Err(v)
        }

        unsafe{
            self.head.write(v);
            self.head=self.head.add(1);
        }

        Ok(())
    }

    pub fn pop(&mut self) -> Option<T> {
        if self.head == self.base {
            return None;
        }

        unsafe {
            self.head=self.head.sub(1);
            let ans = self.head.read();
            Some(ans)
        }
    }

    #[inline]
    pub fn peek<'b>(&'b self) -> Option<&'b T>{
         if self.head == self.base {
            return None;
        }

        unsafe {
            Some(&*self.head.sub(1))
        }
    }

    #[inline]
    pub fn peek_n<'b,const SIZE:usize>(&'b self) -> Option<&'b [T;SIZE]>{
        //we cant do normal arithmetic since it may overflow
         if (self.head as usize) < self.base as usize + SIZE*size_of::<T>(){
            return None;
        }

        unsafe {
            let p = self.head.sub(SIZE) as *const T as *const [T;SIZE];
            Some(&*p)
        }
    }

    #[inline]
    pub fn peek_many<'b>(&'b self,size:usize) -> Option<&'b [T]>{
        //we cant do normal arithmetic since it may overflow
         if (self.head as usize) < self.base as usize + size*size_of::<T>(){
            return None;
        }

        unsafe {
            let p = self.head.sub(size) as *const T;
            Some(slice::from_raw_parts(p,size))
        }
    }
}



#[test]
fn test_push_pop_basic() {
    let mut storage = make_storage::<u32, 4>();
    let mut stack = StackRef::from_raw(&mut storage);

    assert_eq!(stack.pop(), None);

    assert!(stack.push(10).is_ok());
    assert!(stack.push(20).is_ok());

    assert_eq!(stack.pop(), Some(20));
    assert_eq!(stack.pop(), Some(10));
    assert_eq!(stack.pop(), None);
}

#[test]
fn test_push_overflow() {
    let mut storage = make_storage::<u32, 2>();
    let mut stack = StackRef::from_raw(&mut storage);

    assert!(stack.push(1).is_ok());
    assert!(stack.push(2).is_ok());

    // This should fail, stack is full
    assert_eq!(stack.push(3), Err(3));
}

#[test]
fn test_lifo_order() {
    let mut storage = make_storage::<&'static str, 3>();
    let mut stack = StackRef::from_raw(&mut storage);

    stack.push("first").unwrap();
    stack.push("second").unwrap();
    stack.push("third").unwrap();

    assert_eq!(stack.pop(), Some("third"));
    assert_eq!(stack.peek(), Some("second").as_ref());
    assert_eq!(stack.pop(), Some("second"));
    assert_eq!(stack.pop(), Some("first"));
    assert_eq!(stack.pop(), None);
}

#[test]
fn test_peek_n() {
    let mut storage = make_storage::<u32, 5>();
    let mut stack = StackRef::from_raw(&mut storage);

    assert!(stack.push(1).is_ok());
    assert!(stack.push(2).is_ok());
    assert!(stack.push(3).is_ok());
    assert!(stack.push(4).is_ok());
    assert!(stack.push(5).is_ok());

    // Valid peek of top 3 elements
    if let Some(slice) = stack.peek_n::<3>() {
        assert_eq!(slice, &[3, 4, 5]);
    } else {
        panic!("peek_n::<3> should have succeeded");
    }

    // Invalid peek: requesting more than available
    assert!(stack.peek_n::<6>().is_none());
    assert!(stack.peek_n::<5>().is_some());

    // Pop one and peek 3 again: still valid
    assert_eq!(stack.pop(), Some(5));
    assert!(stack.peek_n::<3>().is_some());

    // Pop another and try peeking 3: now too few
    assert_eq!(stack.pop(), Some(4));
    assert!(stack.peek_n::<4>().is_none());
}

use core::mem::size_of;

#[test]
fn test_peek_many() {
    let mut storage = make_storage::<u32, 6>();
    let mut stack = StackRef::from_raw(&mut storage);

    for i in 1..=5 {
        assert!(stack.push(i).is_ok());
    }

    // Valid peek of top 3 elements
    if let Some(slice) = stack.peek_many(3) {
        assert_eq!(slice, &[3, 4, 5]);
    } else {
        panic!("peek_many(3) should have succeeded");
    }

    // Invalid peek: too many elements
    assert!(stack.peek_many(6).is_none());

    // Peek whole stack: should succeed with 5
    if let Some(slice) = stack.peek_many(5) {
        assert_eq!(slice, &[1, 2, 3, 4, 5]);
    } else {
        panic!("peek_many(5) should have succeeded");
    }

    // Pop 2 items and then try peek_many(4) — should fail now
    stack.pop();
    stack.pop();
    assert!(stack.peek_many(4).is_none());

    // Now peek_many(3) should succeed
    if let Some(slice) = stack.peek_many(3) {
        assert_eq!(slice, &[1, 2, 3]);
    } else {
        panic!("peek_many(3) should still work");
    }
}
