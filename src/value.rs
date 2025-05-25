use crate::stack::take_last_raw;
use core::ptr;
use crate::stack::take_last;
use crate::stack::take_last_mut;
use crate::stack::StackRef;



#[derive(Debug,Clone,Copy,PartialEq)]
pub enum ValueTag {
	Int(i64),
	Float(f64),
	Nil,
	Bool(bool),
	Token(u16),
	Code(u64),

	Cons(usize),
	Func(usize),
}

impl ValueTag{
	pub fn get_size(self) -> usize {
		match self {
			ValueTag::Int(_) |
			ValueTag::Float(_) |
			ValueTag::Token(_) | ValueTag::Code(_) |
			ValueTag::Nil | ValueTag::Bool(_) 
			=> {1},
			
			ValueTag::Cons(u) | ValueTag::Func(u) => u+1,
		}
	}
}

pub type ValueStack<'a> = StackRef<'a, ValueTag>;

#[derive(Debug)]
pub enum Error {
	StackOverflow,
	TypeError,
}

pub fn swap_things(stack:&mut ValueStack)-> Result<(),()>{
	let (room,mut temp) = stack.split();
	let room_raw = room as *mut [_];

	let first = room.last().ok_or(())?;
	let first = take_last(room,first.get_size());
	
	assert!(first.len() < room.len());

	let second_size = take_last(room,first.len()+1)[0].get_size();
	let second = &take_last(room,first.len()+second_size)[0..second_size];
	temp.push_slice(second)?;

	let room = take_last_raw(room_raw,second.len()+first.len())  as *mut ValueTag;
	let first = first as *const [_];
	let second = temp.peek_many(second.len()).unwrap() as *const [_];

	unsafe{
		ptr::copy(first as *const ValueTag,room,first.len());
		ptr::copy_nonoverlapping(second as *const ValueTag,room.add(first.len()),second.len());
	}

	Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ValueTag::*;
    use crate::stack::{make_storage, StackRef};


    /* 1. swap two single-element objects ................................... */
    #[test]
    fn swap_two_scalars() {
    	let mut storage = make_storage::<_,4>();
        let mut stack = StackRef::from_slice(&mut storage);

        stack.push_slice(&[Int(1)]).unwrap();   // older  (lower)
        stack.push_slice(&[Int(2)]).unwrap();   // newer  (top)

        swap_things(&mut stack).unwrap();

        assert_eq!(stack.pop(), Some(Int(1)));  // now top
        assert_eq!(stack.pop(), Some(Int(2)));
        assert_eq!(stack.pop(), None);
    }

    /* 2. second is larger than first ....................................... */
    #[test]
    fn swap_vector_and_scalar() {
        let mut storage = make_storage::<_,8>();
        let mut stack = StackRef::from_slice(&mut storage);

        // second (older) = 2-element payload + tag
        stack.push_slice(&[Int(10), Int(11), Cons(2)]).unwrap();

        // first (newer)  = single Int
        stack.push_slice(&[Int(99)]).unwrap();

        swap_things(&mut stack).unwrap();

        assert_eq!(stack.pop(), Some(Cons(2)));   // old second
        assert_eq!(stack.pop(), Some(Int(11)));
        assert_eq!(stack.pop(), Some(Int(10)));
        assert_eq!(stack.pop(), Some(Int(99)));   // old first
        assert_eq!(stack.pop(), None);
    }

    /* 3. both variable-length .............................................. */
    #[test]
    fn swap_two_vectors() {
        let mut storage = make_storage::<_,12>();
        let mut stack = StackRef::from_slice(&mut storage);

        // second (older) – length 4
        stack.push_slice(&[Int(1), Int(2), Int(3), Cons(3)]).unwrap();

        // first (newer)  – length 3
        stack.push_slice(&[Float(4.0), Float(5.0), Cons(2)]).unwrap();

        swap_things(&mut stack).unwrap();

        // Pop and check order

        // old second (payload)
        assert_eq!(stack.pop(), Some(Cons(3)));
        assert_eq!(stack.pop(), Some(Int(3)));
        assert_eq!(stack.pop(), Some(Int(2)));
        assert_eq!(stack.pop(), Some(Int(1)));

        // old first
        assert_eq!(stack.pop(), Some(Cons(2)));
        assert_eq!(stack.pop(), Some(Float(5.0)));
        assert_eq!(stack.pop(), Some(Float(4.0)));    

        assert_eq!(stack.pop(), None);
    }
}


// struct PointerStore<'a,'mem>(&'a mut ValueStack<'mem>);
// impl PointerStore{
// 	fn check_existing(&self,:*mut ValueStack) -> Option<*mut ValueTag>{
// 		todo!()
// 	}
// }

/*
 * the basic idea for storage is we temporarily leak memory willy nilly
 * later we do a GC scan (either on the entire stack or just a span)
 * 
 * that scan lets us squash things down,
 * we temporarily hold the old values in a helper
 * 
 * 
 * 
 * ---------------------------------
 * xxxJUNKxxxAxxJUNKxxx&AB&AC
 * ---------------------------------
 * 
 * helper stack
 * ----------------
 * 
 * ----------------
 * 
 * we start by marking a new spot for A
 * 
 * =>
 * 
 * --------------------------------
 * *AxJUNKxxx&*AxxJUNKxxx&AB&AC
 * --------------------------------
 * 
 * helper stack
 * ---------------
 * &*A A
 * ---------------
 * 
 * we use the value stored in A for fixing all the pointers
 * if we A requires no moving *A is simply chosen to be A
 * 
 * --------------------------------
 * *AxJUNKxxx&*AxxJUNKxxx&A*B&A*C
 * --------------------------------
 * 
 * then we can move things down.
 * note we must skip over *A which we can do based on the helper stack
 * 
 * --------------------------------
 * *A&*AB&*AC
 * --------------------------------
 * 
 * finally NOW we write the value of A to *A
 * this can only be done now because previously A heled *A&
 * so if A == *A the room was already full
 * luckily the helper stack lets us do this in O(1)
 * 
 * 
 * --------------------------------
 * A&*AB&*AC
 * --------------------------------
 * 
 * or in other words 
 * --------------------------------
 * A&AB&AC
 * --------------------------------
 * 
 * this logic can trigger every stack overflow
*/

// /// traces the stack and gcs it
// /// this function can go OOM as it is tracing in which case an OK would be returned
// pub fn gc_the_stack(stack:&mut ValueStack) -> Result<(),()>{
// unsafe{
// 	//step 1 trace
// 	let (alloced,helper) = stack.split();

// 	let _helper = helper.to_slice() as *mut [_] as *mut [ValueTag];

// 	let alloced = alloced as *mut [ValueTag];
// 	if alloced.is_empty() {
// 		return Ok(());
// 	}
// 	let bottom = alloced as *mut ValueTag;
// 	let first = bottom.add(alloced.len()-1);
// 	let mut p = first;

// 	loop {
// 		match *p {
// 			ValueTag::Int(_)
// 			| ValueTag::Float(_)
// 			| ValueTag::Token(_)
// 			| ValueTag::Code(_)
// 			=> {},
			
// 			ValueTag::Cons(_) => todo!(),
// 			ValueTag::Func(_) => todo!(),
// 			ValueTag::Ref(_) => todo!(),

// 		}

// 		if p == bottom {
// 			break;
// 		}
// 		p = p.sub(1);
// 	}

// 	todo!()
// }
// }