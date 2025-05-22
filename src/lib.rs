#![no_std]

use core::ptr;
use core::slice;
use core::cell::UnsafeCell;
use core::mem::MaybeUninit;

pub type Raw<T> = UnsafeCell<MaybeUninit<T>>;
pub fn new_chunk<const SIZE : usize, T>() -> [Raw<T>;SIZE]{
    [const { UnsafeCell::new(MaybeUninit::uninit()) };SIZE]

}

pub struct StackRef<'a,T>{
    base: &'a[Raw<T>],
    head: &'a[Raw<T>],

}

impl<'a, T> StackRef<'a, T>{
    pub fn from_storage(s:&'a[Raw<T>]) -> Self {
        Self{
            base:&s[0..0],
            head:s,
        }
    }

    pub fn pop(&mut self) -> Option<T>{
        unsafe {
            let spot = self.base.last()?;
            let x = ptr::replace(spot.get(),MaybeUninit::uninit());
            
            self.base = &self.base[..self.base.len()-1];
            self.head = slice::from_raw_parts(spot,self.head.len()+1);

            Some(x.assume_init())
        }
        
    }

    pub fn push(&mut self,v:T) -> Result<(),T>{
        unsafe{
            if self.head.is_empty() {
                return Err(v)
            }
            self.head = &self.head[1..self.head.len()];
            self.base = slice::from_raw_parts(self.base.as_ptr(),self.base.len()+1);

            let p = self.base.last().unwrap_unchecked().get();
            ptr::replace(p,MaybeUninit::new(v));
            Ok(())
        }
    }
}



#[test]
fn test_stack_push_pop() {
    let storage = new_chunk::<4, i32>();
    let mut stack = StackRef::from_storage(&storage);

    assert_eq!(stack.pop(), None); // Underflow

    assert_eq!(stack.push(10), Ok(()));
    assert_eq!(stack.push(20), Ok(()));
    assert_eq!(stack.push(30), Ok(()));
    assert_eq!(stack.push(40), Ok(()));
    assert_eq!(stack.push(50), Err(50)); // Overflow

    assert_eq!(stack.pop(), Some(40));
    assert_eq!(stack.pop(), Some(30));
    assert_eq!(stack.pop(), Some(20));
    assert_eq!(stack.pop(), Some(10));
    assert_eq!(stack.pop(), None); // Underflow again
}

#[test]
fn test_stack_ordering() {
    let storage = new_chunk::<2, &str>();
    let mut stack = StackRef::from_storage(&storage);

    assert!(stack.push("first").is_ok());
    assert!(stack.push("second").is_ok());
    assert!(stack.push("third").is_err()); // Overflow

    assert_eq!(stack.pop(), Some("second"));
    assert_eq!(stack.pop(), Some("first"));
    assert_eq!(stack.pop(), None);
}
