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
pub struct ValueStack<'mem,'v>(StackRef<'mem,Value<'v>>);

impl<'mem,'v> ValueStack<'mem,'v>{
	pub fn push_frame(& mut self,v:&[Value<'v>]) -> Result<(),()>{
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
	pub fn peek_frame<'a>(&'a self) -> Option<&'a [Value<'a>]>{
		let Some(Value::Frame(size)) =  self.0.peek() 
		else {
			return None;
		};

		self.0.peek_many(1+size).map(|a| &a[0..*size])
	}

	pub fn push_dependent<'c, F,const SIZE : usize>(&'c mut self,f:F) ->Result<(),[Value<'c>;SIZE]>
	where F:for<'b> FnOnce(&'b [Value<'v>])->[Value<'b>;SIZE]{
		let num_wrote = {
			let (left,right) = self.0.split();
			let vals = f(&left[0..left.len()-1]);

			/*
			 * we are doing a lifetime cast here which seems very odd
			 * it is kinda tricky to see why this safe
			 * but it comes from the core invriance of the stack
			 *
			 * note that F can not leak any refrences
			 * and F can not make any assumbtions about the lifetime (since b is generic)
			*/
			let vals : [Value<'v>;SIZE] = unsafe {core::mem::transmute(vals)};
			let mut s =ValueStack(right);
			s.push_frame_const::<SIZE>(vals)?;
			s.0.write_index()
		};
		unsafe{
			self.0.advance(num_wrote);
		}
		Ok(())
	}

	pub fn call_split<'b_real, F>(&mut self,f:F) ->Result<(),()>
	where 
	'v:'b_real,
	'mem:'b_real,
	F:for<'b> FnOnce(&'b [Value<'b>],&mut ValueStack<'_,'b>) ->Result<(),()>{
		/*
		 * similar idea to push_dependent
		 * note that the cast here discards the mut semantics out of our inner stack
		 * this is intentional
		 *
		 * also note we are not allowing the closure to know the actual liftime of 'b_real
		 * this is because we do not want to allow the closure to leak anything
		 * because that memory could be invalidated on our next move
		*/
		let s : &mut ValueStack<'_,'b_real> = unsafe{core::mem::transmute(&mut *self)};
		
		let (left,right) = s.0.split();
		let mut s =ValueStack(right);
		
		let res = f(&left[0..left.len()-1],&mut s);
		let num_wrote = s.0.write_index();

		unsafe{
			self.0.advance(num_wrote);
		}
		res
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

#[test]
fn test_value_stack_call_split() {
    let mut storage = make_storage::<Value, 10>();
    let mut stack = ValueStack(StackRef::from_slice(&mut storage));

    let a = Value::Int(10);
    let b = Value::Int(20);

    // Push initial frame
    assert!(stack.push_frame(&[a, b]).is_ok());

    // Now invoke call_split to construct a new frame that depends on the current one
    let result = stack.call_split(|input, out_stack| {
        assert_eq!(input, &[a, b]); // left is full frame (excluding the Frame marker)
        let cons = Value::Cons(&input[0], &input[1]);
        out_stack.push_frame_const([cons])
        .map_err(|_|())
    });

    assert!(result.is_ok());

    // Verify the top frame is the one pushed inside `call_split`
    let top = stack.peek_frame().expect("Expected a top frame after call_split");
    match top {
        [Value::Cons(Value::Int(10), Value::Int(20))] => {},
        _ => panic!("Unexpected top frame: {:?}", top),
    }

    // Cleanup both frames
    assert!(stack.drop_frame().is_ok());
    assert!(stack.drop_frame().is_ok());

    assert!(stack.peek_frame().is_none());
}

