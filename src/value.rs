use crate::stack::make_storage;
use crate::stack::StackRef;

#[derive(Debug,Clone,Copy,PartialEq)]
pub enum Value<'a>{
	Frame(usize),

	Nil,
	False,
	True,
	Int(i64),
	Cons(&'a Value<'a>,&'a Value<'a>),
}

/// this type provies a SAFE abstraction over the value stack.
/// it is the main place where we have to deal with the crazy unsafety of what this crate does.
/// 
/// the key invriance to keep in mind is values can only refrence things in frames BELOW them
/// this ensures that we can safely pop the top of the stack and overwrite it
/// the partision into frames is here to allow poping of multiple values
/// it is UNSOUND to pop more than 1 frame at a time
/// once the bottom frame is poped the top refrence is invalidated
pub struct ValueStack<'a,'v>(StackRef<'a,Value<'v>>);

impl<'a,'v> ValueStack<'a,'v>{
	pub fn push_frame(&mut self,v:&[Value<'v>]) -> Result<(),()>{
		self.0.push_slice(v)?;
		self.0.push(Value::Frame(v.len()))
		.map_err(|_| self.0.flush(v.len()))?;
		Ok(())
	}

	pub fn push_frame_const<const SIZE:usize>(&mut self,v:[Value<'v>;SIZE]) -> Result<(),[Value<'v>;SIZE]>{
		self.0.push_n::<SIZE>(v)?;
		self.0.push(Value::Frame(SIZE))
		.map_err(|_| self.0.pop_n::<SIZE>().unwrap())?;
		Ok(())
	}

	#[inline]
	pub fn peek_frame(&self) -> Option<&[Value<'v>]>{
		let Some(Value::Frame(size)) =  self.0.peek() 
		else {
			return None;
		};

		self.0.peek_many(1+size).map(|a| &a[0..*size])
	}

	pub fn push_dependent<F,const SIZE : usize>(&mut self,f:F) ->Result<(),()>
	where F:for<'b> FnOnce(&'b [Value<'v>])->[Value<'b>;SIZE]{
		let num_wrote = {
			let (left,right) = self.0.split();
			let vals = f(&left[0..left.len()-1]);

			/*
			 * we are doing a lifetime cast here which seems very odd
			 * it is kinda tricky to see why this safe
			 * but it comes from the core invriance of the stack
			*/
			let vals = unsafe {
				let p = &vals as *const [Value<'_>] as *const [Value<'v>];
				&*p
			};
			let mut s =ValueStack(right);
			s.push_frame(vals)?;
			s.0.write_index()
		};
		unsafe{
			self.0.advance(num_wrote);
		}
		Ok(())
	}

	pub fn drop_frame(&mut self) -> Result<(),()>{
		let Some(Value::Frame(size)) =  self.0.peek() 
		else {
			return Err(());
		};

		self.0.flush(size+1);
		Ok(())
	}
}

#[test]
fn double_read_on_copy(){
	let x = 42;
    let ptr = &x as *const i32;

    unsafe {
        let a = ptr.read(); // OK: i32 is Copy
        let b = ptr.read(); // Also OK
        let _c = a+b;
    }
}

#[test]
fn test_value_stack_push_peek_drop_frame() {
    let mut storage = make_storage::<Value, 10>();
    let mut stack = ValueStack(StackRef::from_slice(&mut storage));

    let a = Value::Int(1);
    let b = Value::Int(2);

    assert!(stack.push_frame(&[a, b]).is_ok());

    // Check peek_frame sees top frame
    let peeked = stack.peek_frame().expect("Expected valid frame");
    assert_eq!(peeked, &[a, b]);

    // Now push a dependent frame that references the previous values
    assert!(stack.push_dependent(|frame| {
        assert_eq!(frame, &[a, b]);
        let cons = Value::Cons(&frame[0], &b);
        [cons]
    }).is_ok());

    // Check new frame is top
    let top = stack.peek_frame().expect("Expected dependent frame");
    match top {
        [Value::Cons(Value::Int(1), Value::Int(2))] => {},
        _ => panic!("Unexpected frame content: {:?}", top),
    }

    // Drop top frame, should restore the previous one
    assert!(stack.drop_frame().is_ok());

    let after_drop = stack.peek_frame().expect("Expected frame after drop");
    assert_eq!(after_drop, &[a, b]);

    // Drop again to empty the stack
    assert!(stack.drop_frame().is_ok());

    // Should now be empty
    assert!(stack.peek_frame().is_none());
}
