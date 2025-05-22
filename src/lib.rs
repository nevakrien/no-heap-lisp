#![no_std]

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
    assert_eq!(stack.pop(), Some("second"));
    assert_eq!(stack.pop(), Some("first"));
    assert_eq!(stack.pop(), None);
}
