use core::ptr;
use core::slice;
use core::mem::MaybeUninit;
use core::marker::PhantomData;

pub fn make_storage<T,const SIZE:usize>() ->[MaybeUninit<T>;SIZE]{
    [const { MaybeUninit::uninit() };SIZE]
}

pub fn take_last<T>(slice: &[T], n: usize) -> &[T] {
    let len = slice.len();
    let start = len.saturating_sub(n);
    &slice[start..]
}

pub fn take_last_mut<T>(slice: &mut [T], n: usize) -> &mut [T] {
    let len = slice.len();
    let start = len.saturating_sub(n);
    &mut slice[start..]
}

pub fn take_last_raw<T:Sized>(slice: *mut [T], n: usize) -> *mut [T] {
    let len = slice.len();
    let start = len.saturating_sub(n);
    let p = unsafe{(slice as *mut T).add(start)};

    ptr::slice_from_raw_parts_mut(p,len-start)
}

#[test]
fn test_take_last() {
    let slice = [10, 20, 30, 40, 50];

    assert_eq!(take_last(&slice, 0), &[]);
    assert_eq!(take_last(&slice, 2), &[40, 50]);
    assert_eq!(take_last(&slice, 5), &[10, 20, 30, 40, 50]);
    assert_eq!(take_last(&slice, 10), &[10, 20, 30, 40, 50]); // overflow-safe
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
    #[inline]
    pub fn new_full(mem:&'a mut [T]) -> Self{
        let base = mem.as_mut_ptr();
        let end = unsafe {base.add(mem.len())}; 
        Self{
            base,
            head: end,
            end,

            _phantom:PhantomData,
        }
    }

    #[inline]
    pub fn from_slice(mem:&'a mut [MaybeUninit<T>]) -> Self{
        let base = mem.as_mut_ptr() as _;
        Self{
            base,
            head: base,
            end: unsafe {base.add(mem.len())},

            _phantom:PhantomData,
        }
    }

    #[inline]
    pub fn to_slice(self) -> &'a mut [MaybeUninit<T>] {
        unsafe { 
            let len = self.end.offset_from(self.base) as usize; 
            let p = ptr::slice_from_raw_parts_mut(self.base,len);
            &mut *(p as *mut [MaybeUninit<T>])
        }
    }

    #[inline]
    pub fn as_slice<'b>(&'b mut self)-> &'b mut [MaybeUninit<T>] {
        unsafe { 
            let len = self.end.offset_from(self.base) as usize; 
            let p = ptr::slice_from_raw_parts_mut(self.base,len);
            &mut *(p as *mut [MaybeUninit<T>])
        }
    }

    #[inline]
    pub fn room_left(&self) -> usize {
        unsafe { 
            self.end.offset_from(self.head).try_into().unwrap() 
        } 
    }

    /// returns the index the index the writing head points to
    /// [T T T |*****junk****]
    ///        ^
    #[inline]
    pub fn write_index(&self) -> usize {
        unsafe { self.head.offset_from(self.base) as usize }
    }

    /// sets the write index retrived from write_index
    /// note that the memory below that index is assumed inilized 
    #[inline    ]
    pub unsafe fn set_write_index(&mut self,idx:usize){ unsafe {
        self.head = self.base.add(idx)
    }}

    #[inline]
    pub unsafe fn advance(&mut self,add:usize){ unsafe {
        self.set_write_index(self.write_index()+add)
    }}

    /// splits the stack into a full left part and an empty right part
    pub fn split<'b>(&'b mut self) -> (&'b mut [T],StackRef<'b, T>){
        let end = StackRef{
            base:self.head,
            head:self.head,
            end:self.end,

            _phantom:PhantomData,

        };

        // let start = StackRef{
        //     base:self.base,
        //     head:self.head,
        //     end:self.head,

        //     _phantom:PhantomData,

        // };
        let s = unsafe { 
            let len = self.head.offset_from(self.base) as usize; 
            &mut*ptr::slice_from_raw_parts_mut(self.base,len)
        };

        (s,end)
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

    pub fn push_n<const SIZE:usize>(&mut self,v:[T;SIZE]) -> Result<(),[T;SIZE]> {
        if SIZE == 0 {
            return Ok(())
        }
        //pointer arithmetic can overflow here
        if self.head as usize + (SIZE-1)*size_of::<T>() >= self.end as usize {
            return Err(v)
        }

        unsafe{
            (self.head as *mut [T;SIZE]).write(v);
            self.head=self.head.add(SIZE);
        }

        Ok(())
    }

    pub fn push_slice(&mut self,v:&[T]) -> Result<(),()>
    where T : Clone {
        //pointer arithmetic can overflow here
        if self.head as usize + (v.len()-1)*size_of::<T>() >= self.end as usize {
            return Err(())
        }

        unsafe{
            let spot = &mut*ptr::slice_from_raw_parts_mut(self.head,v.len());
            spot.clone_from_slice(v);
            self.head=self.head.add(v.len());
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
    pub fn pop_many<'b>(&'b mut self,size:usize) -> Option<&'b mut [T]>
    where T :Copy
    {
        //we cant do normal arithmetic since it may overflow
         if (self.head as usize) < self.base as usize + size*size_of::<T>(){
            return None;
        }

        unsafe {
            self.head=self.head.sub(size);
            let p = self.head as *mut T;
            Some(&mut*ptr::slice_from_raw_parts_mut(p,size))
        }
    }

    //drops starting from skip below the counter taking count upward
    pub fn drop_inside(&mut self,skip:usize,count:usize)-> Result<(),()>{
        if skip == 0 {
            return Ok(())
        }

        let stack_end = self.write_index().checked_sub(1).ok_or(())?;
        let spot = stack_end.checked_sub(skip).ok_or(())?;
        let start_good = spot.checked_add(count).ok_or(())?;

        if start_good > stack_end{
            return Err(())
        }

        let count_move = self.write_index() - start_good;

        unsafe{
            let p_start = self.base.add(spot);
            let p_good = self.base.add(start_good);
            ptr::copy(p_good as *const _,p_start,count_move);

            self.head=self.head.sub(count);
        }

        Ok(())

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

    pub fn flush_all(&mut self){
        for _ in self.into_iter() {

        }
    }

    pub fn flush(&mut self,len:usize){
        for _ in 0..len {
            self.pop();
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

#[test]
fn test_split_stack() {
    let mut storage = make_storage::<u32, 6>();
    let mut original = StackRef::from_slice(&mut storage);

    // Push three elements
    assert!(original.push(1).is_ok());
    assert!(original.push(2).is_ok());
    assert!(original.push(3).is_ok());

    // Split the stack
    let (left, mut right) = original.split();

    // Left stack should contain [1, 2, 3]
    assert_eq!(left, [1,2,3]);

    // Right stack should be empty
    assert_eq!(right.pop(), None);

    // Push to right and check it's isolated from left
    assert!(right.push(10).is_ok());
    assert_eq!(right.pop(), Some(10));
}

#[test]
fn test_push_slice_success_and_error() {
    let mut storage = make_storage::<u32, 5>();
    let mut stack = StackRef::from_slice(&mut storage);

    // This slice fits
    let input1 = [1, 2, 3];
    assert_eq!(stack.push_slice(&input1), Ok(()));
    assert_eq!(stack.peek_many(3), Some(&[1, 2, 3][..]));

    // This slice would overflow (only 2 slots left)
    let input2 = [4, 5, 6];
    assert_eq!(stack.push_slice(&input2), Err(()));

    // Push exactly remaining capacity
    let input3 = [4, 5];
    assert_eq!(stack.push_slice(&input3), Ok(()));

    // Stack should now be full
    assert_eq!(stack.write_index(), 5);
    assert!(stack.push_slice(&[99]).is_err());

    stack.pop().unwrap();
    assert!(stack.push_slice(&[99,66]).is_err());

    stack.pop().unwrap();
    assert!(stack.push_slice(&[99,66,11]).is_err());

}

#[test]
fn test_weird_write_error(){
    let mut storage = make_storage::<i64, 6>();
    let mut stack   = StackRef::from_slice(&mut storage);

    stack.push_slice(&[2]).unwrap();
    stack.push_n([1]).unwrap();

    stack.push_slice(&[2,3]).unwrap();
    stack.push_n([2]).unwrap();

    stack.push_slice(&[1,2,3]).unwrap_err();
}

#[test]
fn test_full_usage() {
    let mut data = [10, 20, 30, 40,50];
    let mut stack = StackRef::new_full(&mut data);

    // At construction, all items should be present in reverse push order
    assert_eq!(stack.write_index(), 5);
    assert_eq!(stack.room_left(), 0);

    // Peek top item
    assert_eq!(stack.peek(), Some(&50));

    // Pop all elements, expect LIFO order
    stack.drop_inside(3,2).unwrap();
    assert_eq!(stack.room_left(), 2);


    assert_eq!(stack.pop(), Some(50));
    assert_eq!(stack.pop(), Some(40));
    assert_eq!(stack.room_left(), 4);


    
    // assert_eq!(stack.pop(), Some(30));
    // assert_eq!(stack.pop(), Some(20));

    assert_eq!(stack.pop(), Some(10));
    assert_eq!(stack.room_left(), 5);

    // Now empty
    assert_eq!(stack.pop(), None);

    // Push again to see if reuse is correct
    assert!(stack.push(77).is_ok());
    assert_eq!(stack.peek(), Some(&77));
    assert!(stack.push(77).is_ok());

    stack.pop_many(4).ok_or(()).unwrap_err();
    stack.pop_many(2).unwrap();
}
