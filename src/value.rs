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

/// This type provides a SAFE abstraction over the value stack.
/// It is the main place where we have to deal with the unsafe behavior of this crate.
///
/// The key invariant to keep in mind is that values can only reference things in frames BELOW them
/// This ensures that we can safely pop the top of the stack and overwrite it
/// The partition into frames is here to allow popping of multiple values
///
/// It is UNSOUND to pop more than 1 frame at a time
///
/// Once the bottom frame is popped, the top reference is invalidated
/// This is captured by the public API
pub struct ValueStack<'mem,'v>(StackRef<'mem,Value<'v>>);

impl<'mem,'v> ValueStack<'mem,'v>{
	// !!!never write this!!!!
	// pub fn peek_all_long<'a>(&'a self) -> &'a [Value<'v>]{
	// 	self.0.peek_many(self.0.write_index()).unwrap()
	// }
	// yes its "safe" no realising a 'v is unsound dont

	pub fn new(s:StackRef<'mem,Value<'v>>) -> Self{
		Self(s.into())
	}
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

	/// This gives a reference to the top stack frame
	/// Note the lifetime does have to be 'a here because 
	/// popping and then writing over values is possible
	#[inline]
	pub fn peek_frame<'a>(&'a self) -> Option<&'a [Value<'a>]>{
		let Some(Value::Frame(size)) =  self.0.peek() 
		else {
			return None;
		};

		self.0.peek_many(1+size).map(|a| &a[0..*size])
	}

	/// This gives a reference to the entire stack
	/// Note the lifetime does have to be 'a here because 
	/// popping and then writing over values is possible
	#[inline]
	pub fn peek_all<'a>(&'a self) -> &'a [Value<'a>]{
		self.0.peek_many(self.0.write_index()).unwrap()
	}

	pub fn push_dependent<'c, F,const SIZE : usize>(&'c mut self,f:F) ->Result<(),[Value<'c>;SIZE]>
	where F:for<'b> FnOnce(&'b [Value<'v>])->[Value<'b>;SIZE]{
		let (left,right) = self.0.split();
		let vals = f(left);

		/*
		 * We are doing a lifetime cast here, which seems very odd
		 * It is kinda tricky to see why this is safe
		 * but it comes from the core invariance of the stack
		 *
		 * Note that F can not make any assumptions about the lifetime (since b is generic)
		*/
		let vals : [Value<'v>;SIZE] = unsafe {core::mem::transmute(vals)};
		let mut s =ValueStack::new(right);
		s.push_frame_const::<SIZE>(vals)?;
		
		let num_wrote=s.0.write_index();
		unsafe{
			self.0.advance(num_wrote);
		}
		Ok(())
	}

	/// runs a function on the stack appending all if the returned value
	/// all values from the returned stack are appended into the main stack
	/// this is used for apapending a variable length frame refrencing the current stack
	pub fn call_split<'b_real, F>(&mut self,f:F) ->Result<(),()>
	where 
	'v:'b_real,
	'mem:'b_real,

	F:for<'b> FnOnce(&'b [Value<'b>],&mut ValueStack<'_,'b>) ->Result<(),()>{
		/*
		 * similar idea to push_dependent
		 * Note that the cast here discards the mut semantics out of our inner stack
		 * This is intentional
		 *
		 * Also note, we are not allowing the closure to know the actual lifetime of 'b_real
		 * This is because we do not want to allow the closure to leak anything
		 * because that memory could be invalidated on our next move
		*/
		let s : &mut ValueStack<'_,'b_real> = unsafe{core::mem::transmute(&mut *self)};
		
		let (left,right) = s.0.split();
		let mut s =ValueStack::new(right);
		
		let res = f(left,&mut s);
		let num_wrote = s.0.write_index();

		unsafe{
			self.0.advance(num_wrote);
		}
		res
	}

	/// similar to call_split but also pops the current stack frame
	/// while its not possible to refrence tthat stack frame directly
	/// all values in it can be copied
	pub fn call_split_drop<'b_real, F>(&mut self,f:F) ->Result<(),()>
	where 
	'v:'b_real,
	'mem:'b_real,

	F:for<'b> FnOnce(&'b [Value<'b>],&[Value<'b>],&mut ValueStack<'_,'b>) ->Result<(),()>{
		/*
		 * similar idea to push_dependent
		 * Note that the cast here discards the mut semantics out of our inner stack
		 * This is intentional
		 *
		 * Also note, we are not allowing the closure to know the actual lifetime of 'b_real
		 * This is because we do not want to allow the closure to leak anything
		 * because that memory could be invalidated on our next move
		*/
		let s : &mut ValueStack<'_,'b_real> = unsafe{core::mem::transmute(&mut *self)};
		

		let (left,right) = s.0.split();
		let mut s =ValueStack::new(right);

		let Some(Value::Frame(size)) = left.last() else{return Err(())};
		let rest_len = left.len()-size-1;
		let rest =&left[..rest_len];
		let temp =&left[rest_len..];

		let res = f(rest,&temp[..*size],&mut s); 

		let frame = s.peek_frame().ok_or(())? as *const [Value<'_>] as *const [Value<'v>];

		unsafe{
			self.0.flush(temp.len());
			self.push_frame(&*frame)?;
		}
		res
	}


	pub fn drop_frame(&mut self) -> Result<(),()>{
		let Some(Value::Frame(size)) = self.0.peek() 
		else {
			return Err(());
		};

		self.0.flush(size+1);
		Ok(())
	}

	#[inline]
	pub fn drop_n(&mut self,n:usize){
		self.0.flush(n)
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
    let mut stack = ValueStack::new(StackRef::from_slice(&mut storage));

    let a = Value::Int(1);
    let b = Value::Int(2);

    assert!(stack.push_frame(&[a, b]).is_ok());

    // Check peek_frame sees top frame
    let peeked = stack.peek_frame().expect("Expected valid frame");
    assert_eq!(peeked, &[a, b]);

    // Now push a dependent frame that references the previous values
    assert!(stack.push_dependent(|frame| {
        assert_eq!(frame, &[a, b,Value::Frame(2)]);
        let cons = Value::Cons(&frame[0], &b);
        [cons]
    }).is_ok());

    // Check the new frame is top
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
    let mut stack = ValueStack::new(StackRef::from_slice(&mut storage));

    let a = Value::Int(10);
    let b = Value::Int(20);

    // Push initial frame
    assert!(stack.push_frame(&[a, b]).is_ok());

    // Now invoke call_split to construct a new frame that depends on the current one
    let result = stack.call_split(|input, out_stack| {
        assert_eq!(input, &[a, b,Value::Frame(2)]); // left is full frame (excluding the Frame marker)
        let cons = Value::Cons(&input[0], &input[1]);
        out_stack.push_frame_const([cons])
        .map_err(|_|())
    });

    assert!(result.is_ok());
    assert_eq!(stack.peek_all(),&[a,b,Value::Frame(2),Value::Cons(&a,&b),Value::Frame(1)]);

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

#[test]
fn test_value_stack_call_split_drop() {
    use Value::*;

    // backing store that is comfortably large
    let mut storage = make_storage::<Value, 16>();
    let mut stack   = ValueStack::new(StackRef::from_slice(&mut storage));

    // ── frame #0 ──────────────────────────────────────────────
    let base = Int(9);
    stack.push_frame(&[base]).unwrap();
    // stack:  Int(9)  Frame(1)

    // ── frame #1 (will be “dropped” inside call_split_drop) ──
    let x = Int(1);
    let y = Int(2);
    stack.push_frame(&[x, y]).unwrap();
    // stack:  Int(9) Frame(1)  Int(1) Int(2) Frame(2)

    // ---------------------------------------------------------
    stack
        .call_split_drop(|rest, frame, inner| {
            // `rest` should be the whole prefix before the frame-to-drop
            assert_eq!(rest, &[base, Frame(1)]);
            assert_eq!(frame, &[x, y]);

            // create a new value that *only* references `rest`,
            // never the soon-to-be-flushed `frame`.
            let cons   = Cons(&rest[0], &rest[0]);

            // push a replacement frame
            inner
                .push_frame_const([cons,frame[1]])
                .map_err(|_| ())
        })
        .unwrap();

    // expected final layout:
    //   Int(9)  Frame(1)           ← original lower frame
    //   Cons(&Int(9), &Int(42))    ← replacement upper frame
    //   Frame(1)
    let expected_cons = Value::Cons(&base,&base);
    assert_eq!(
        stack.peek_all(),
        &[base, Frame(1), expected_cons,y, Frame(2)]
    );

    // top-of-stack sanity
    let top = stack.peek_frame().unwrap();
    assert_eq!(top, &[expected_cons,y]);
}
