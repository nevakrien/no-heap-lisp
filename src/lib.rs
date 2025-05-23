#![no_std]

use core::ptr;
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

    _phantom:PhantomData<&'a mut T>
}

unsafe impl<'a, T: Send> Send for StackRef<'a, T> {}
unsafe impl<'a, T: Sync> Sync for StackRef<'a, T> {}

impl<T> Iterator for StackRef<'_, T>{

type Item = T;
fn next(&mut self) -> Option<T> { self.pop() }
}


impl<'a, T> StackRef<'a, T>{
    pub fn from_slice(mem:&'a mut [MaybeUninit<T>]) -> Self{
        let base = mem.as_mut_ptr() as _;
        Self{
            base,
            head: base,
            end: unsafe {base.add(mem.len())},

            _phantom:PhantomData,
        }
    }

    pub fn to_slice(self) -> &'a mut [MaybeUninit<T>] {
        unsafe { 
            let len = self.end.offset_from(self.base) as usize; 
            let p = ptr::slice_from_raw_parts_mut(self.base,len);
            &mut *(p as *mut [MaybeUninit<T>])
        }
    }

    /// returns the index the index the writing head points to
    /// [T T T |*****junk****]
    ///        ^
    pub fn write_index(&self) -> usize {
        unsafe { self.head.offset_from(self.base) as usize }
    }

    /// sets the write index retrived from write_index
    /// note that the memory below that index is assumed inilized 
    pub unsafe fn set_write_index(&mut self,idx:usize){ unsafe {
        self.head = self.base.add(idx)
    }}

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

    pub fn push_n<const SIZE:usize>(&mut self,v:[T;SIZE]) -> Result<(),[T;SIZE]> {
        //pointer arithmetic can overflow here
        if self.head as usize + (SIZE-1)*size_of::<T>() == self.end as usize {
            return Err(v)
        }

        unsafe{
            (self.head as *mut [T;SIZE]).write(v);
            self.head=self.head.add(SIZE);
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

    pub fn pop_n<const SIZE:usize>(&mut self)->Option<[T;SIZE]>{
        if (self.head as usize) < self.base as usize + SIZE*size_of::<T>(){
            return None;
        }

        unsafe {
            self.head=self.head.sub(SIZE);
            let ans = (self.head as *mut [T;SIZE]).read();
            Some(ans)
        }
    }

    #[inline]
    pub fn peek<'b>(&'b self) -> Option<&'b T>{
        self.peek_n::<1>().map(|a| &a[0])
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

    pub fn flush(&mut self){
        for _ in self.into_iter() {

        }
    }
}

#[test]
fn test_lifo_order() {
    let mut storage = make_storage::<&'static str, 3>();
    let mut stack = StackRef::from_slice(&mut storage);

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
    let mut stack = StackRef::from_slice(&mut storage);

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
    let mut stack = StackRef::from_slice(&mut storage);

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

#[test]
fn test_push_n_and_pop_n_success() {
    let mut storage = make_storage::<u32, 6>();
    let mut stack = StackRef::from_slice(&mut storage);

    let arr1 = [10, 20];
    let arr2 = [30, 40, 50];

    assert!(stack.push_n(arr1).is_ok());
    assert!(stack.push_n(arr2).is_ok());

    // Should pop [30, 40, 50] first
    if let Some(popped) = stack.pop_n::<3>() {
        assert_eq!(popped, [30, 40, 50]);
    } else {
        panic!("pop_n::<3> should succeed");
    }

    // Then pop [10, 20]
    if let Some(popped) = stack.pop_n::<2>() {
        assert_eq!(popped, [10, 20]);
    } else {
        panic!("pop_n::<2> should succeed");
    }

    assert!(stack.pop_n::<1>().is_none());
}

#[test]
fn test_push_n_overflow() {
    let mut storage = make_storage::<u32, 4>();
    let mut stack = StackRef::from_slice(&mut storage);

    let ok = [1, 2];
    let fail = [3, 4, 5];

    assert!(stack.push_n(ok).is_ok());
    assert_eq!(stack.push_n(fail), Err(fail));
}

#[test]
fn test_pop_n_underflow() {
    let mut storage = make_storage::<u32, 3>();
    let mut stack = StackRef::from_slice(&mut storage);

    assert!(stack.push(1).is_ok());
    assert!(stack.pop_n::<2>().is_none());
    assert!(stack.pop_n::<1>().is_some());
    assert!(stack.pop_n::<1>().is_none());
}

#[test]
fn test_mixed_push_pop_n() {
    let mut storage = make_storage::<u32, 6>();
    let mut stack = StackRef::from_slice(&mut storage);

    assert!(stack.push_n([1, 2, 3]).is_ok());
    assert!(stack.push(4).is_ok());
    assert!(stack.push_n([5, 6]).is_ok());

    // pop top 2: should be [5, 6]
    assert_eq!(stack.pop_n::<2>(), Some([5, 6]));

    // pop single: should be 4
    assert_eq!(stack.pop(), Some(4));

    // pop 3: should be [1, 2, 3]
    assert_eq!(stack.pop_n::<3>(), Some([1, 2, 3]));

    // stack is now empty
    assert!(stack.pop().is_none());
}

#[test]
fn test_slice_conversion_basic() {
    let mut storage = make_storage::<u32, 4>();
    let mut stack = StackRef::from_slice(&mut storage);

    assert_eq!(stack.pop(), None);

    assert!(stack.push(10).is_ok());
    assert!(stack.push(20).is_ok());

    let idx = stack.write_index();
    assert_eq!(idx,2);

    let slice = stack.to_slice();
    let mut stack = StackRef::from_slice(slice);
    unsafe {
        stack.set_write_index(idx);
    }

    assert_eq!(stack.pop(), Some(20));
    assert_eq!(stack.pop(), Some(10));
    assert_eq!(stack.pop(), None);
}