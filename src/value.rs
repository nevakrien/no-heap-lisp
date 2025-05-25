// use crate::stack::make_storage;
// use crate::stack::StackRef;

// #[derive(Debug,Clone,Copy,PartialEq)]
// pub enum Value<'a>{
// 	Nil,
// 	False,
// 	True,
// 	Int(i64),
// 	Cons(&'a Value<'a>,&'a Value<'a>),
// }

// /// This type provides a SAFE abstraction over the value stack.
// /// It is the main place where we have to deal with the unsafe behavior of this crate.
// ///
// /// The key invariant to keep in mind is that values can only reference things in frames BELOW them
// /// This ensures that we can safely pop the top of the stack and overwrite it
// ///
// /// It is UNSOUND to pop more than 1 frame at a time
// ///
// /// Once the bottom frame is popped, the top reference is invalidated
// /// This is captured by the public API
// pub struct ValueStack<'mem,'v>(StackRef<'mem,Value<'v>>);

// impl<'mem,'v> ValueStack<'mem,'v>{
// 	// !!!never write this!!!!
// 	// pub fn peek_all_long<'a>(&'a self) -> &'a [Value<'v>]{
// 	// 	self.0.peek_many(self.0.write_index()).unwrap()
// 	// }
// 	// yes its "safe" no realising a 'v is unsound dont

// 	pub fn new(s:StackRef<'mem,Value<'v>>) -> Self{
// 		Self(s.into())
// 	}

// 	pub fn push(&mut self,v:Value<'v>) -> Result<(),Value<'v>>{
// 		self.0.push(v)
// 	}
// 	pub fn push_n<const SIZE:usize>(&mut self,v:[Value<'v>;SIZE]) -> Result<(),[Value<'v>;SIZE]>{
// 		self.0.push_n(v)
// 	}
// 	pub fn push_slice(&mut self,v:&[Value<'v>]) -> Result<(),()>{
// 		self.0.push_slice(v)
// 	}

// 	// Note the lifetime does have to be 'a here because 
// 	// popping and then writing over values is possible
// 	// it is not something rustc can see but please be very mindful

// 	pub fn peek<'a>(&'a self) -> Option<&'a Value<'a>>{
// 		self.0.peek()
// 	}

// 	pub fn peek_n<'a,const SIZE:usize>(&'a self) -> Option<&'a [Value<'a>;SIZE]>{
// 		self.0.peek_n()
// 	}

// 	pub fn peek_many<'a>(&'a self,n:usize) -> Option<&'a [Value<'a>]>{
// 		self.0.peek_many(n)
// 	}

// 	pub fn pop<'a>(&'a mut self) -> Option<Value<'a>>{
// 		self.0.pop()
// 	}

// 	pub fn pop_n<'a,const SIZE:usize>(&'a mut self) -> Option<[Value<'a>;SIZE]>{
// 		self.0.pop_n()
// 	}

// 	/// This gives a reference to the entire stack
// 	#[inline]
// 	pub fn peek_all<'a>(&'a self) -> &'a [Value<'a>]{
// 		self.0.peek_many(self.0.write_index()).unwrap()
// 	}

// 	pub fn push_dependent<'c, F,const SIZE : usize>(&'c mut self,f:F) ->Result<(),[Value<'c>;SIZE]>
// 	where F:for<'b> FnOnce(&'b [Value<'v>])->[Value<'b>;SIZE]{
// 		let (left,right) = self.0.split();
// 		let vals = f(left);

// 		/*
// 		 * We are doing a lifetime cast here, which seems very odd
// 		 * It is kinda tricky to see why this is safe
// 		 * but it comes from the core invariance of the stack
// 		 *
// 		 * Note that F can not make any assumptions about the lifetime (since b is generic)
// 		*/
// 		let vals : [Value<'v>;SIZE] = unsafe {core::mem::transmute(vals)};
// 		let mut s =ValueStack::new(right);
// 		s.push_n::<SIZE>(vals)?;
		
// 		let num_wrote=s.0.write_index();
// 		unsafe{
// 			self.0.advance(num_wrote);
// 		}
// 		Ok(())
// 	}

// 	/// runs a function on the stack appending all if the returned value
// 	/// all values from the returned stack are appended into the main stack
// 	/// this is used for apapending a variable length frame refrencing the current stack
// 	pub fn call_split<'b_real, F>(&mut self,f:F) ->Result<(),()>
// 	where 
// 	'v:'b_real,
// 	'mem:'b_real,

// 	F:for<'b> FnOnce(&'b [Value<'b>],&mut ValueStack<'_,'b>) ->Result<(),()>{
// 		/*
// 		 * similar idea to push_dependent
// 		 * Note that the cast here discards the mut semantics out of our inner stack
// 		 * This is intentional
// 		 *
// 		 * Also note, we are not allowing the closure to know the actual lifetime of 'b_real
// 		 * This is because we do not want to allow the closure to leak anything
// 		 * because that memory could be invalidated on our next move
// 		*/
// 		let s : &mut ValueStack<'_,'b_real> = unsafe{core::mem::transmute(&mut *self)};
		
// 		let (left,right) = s.0.split();
// 		let mut s =ValueStack::new(right);
		
// 		let res = f(left,&mut s);
// 		let num_wrote = s.0.write_index();

// 		unsafe{
// 			self.0.advance(num_wrote);
// 		}
// 		res
// 	}

// 	/// similar to call_split but also pops the current stack frame
// 	/// while its not possible to refrence tthat stack frame directly
// 	/// all values in it can be copied
// 	pub fn call_split_drop<'b_real, F>(&mut self,size:usize,f:F) ->Result<(),()>
// 	where 
// 	'v:'b_real,
// 	'mem:'b_real,

// 	F:for<'b> FnOnce(&'b [Value<'b>],&[Value<'b>],&mut ValueStack<'_,'b>) ->Result<(),()>{
// 		/*
// 		 * similar idea to push_dependent
// 		 * Note that the cast here discards the mut semantics out of our inner stack
// 		 * This is intentional
// 		 *
// 		 * Also note, we are not allowing the closure to know the actual lifetime of 'b_real
// 		 * This is because we do not want to allow the closure to leak anything
// 		 * because that memory could be invalidated on our next move
// 		*/
// 		let s : &mut ValueStack<'_,'b_real> = unsafe{core::mem::transmute(&mut *self)};
		

// 		let (left,right) = s.0.split();
// 		let mut s =ValueStack::new(right);

// 		let rest_len = left.len()-size;
// 		let rest =&left[..rest_len];
// 		let temp =&left[rest_len..];

// 		let res = f(rest,&temp[..size],&mut s); 

// 		let frame = s.peek_many(size).ok_or(())? as *const [Value<'_>] as *const [Value<'v>];
// 		let len = frame.len();
		
// 		//no need to check since what we are writing MUST fit
// 		//since we are just moving
// 		unsafe{
// 			self.0.flush(temp.len());
// 			let mut ptr = frame as *const Value<'v>;
			
// 			for _ in 0..len {
// 				self.0.push(ptr.read()).unwrap();
// 				ptr = ptr.add(1);
// 			}
// 		}
// 		res
// 	}

// 	#[inline]
// 	pub fn flush(&mut self,n:usize){
// 		self.0.flush(n)
// 	}
// }

// #[test]
// fn double_read_on_copy(){
// 	let x = 42;
//     let ptr = &x as *const i32;

//     unsafe {
//         let a = ptr.read(); // OK: i32 is Copy
//         let b = ptr.read(); // Also OK
//         let _c = a+b;
//     }
// }

// #[test]
// fn test_value_stack_push_peek_drop_frame() {
//     let mut storage = make_storage::<Value, 10>();
//     let mut stack = ValueStack::new(StackRef::from_slice(&mut storage));

//     let a = Value::Int(1);
//     let b = Value::Int(2);

//     assert!(stack.push_slice(&[a, b]).is_ok());

//     // Check peek_frame sees top frame
//     let peeked = stack.peek_n::<2>().expect("Expected valid frame");
//     assert_eq!(peeked, &[a, b]);

//     // Now push a dependent frame that references the previous values
//     assert!(stack.push_dependent(|frame| {
//         assert_eq!(frame, &[a, b]);
//         let cons = Value::Cons(&frame[0], &b);
//         [cons]
//     }).is_ok());

//     // Check the new frame is top
//     let top = stack.peek_many(1).expect("Expected dependent frame");
//     match top {
//         [Value::Cons(Value::Int(1), Value::Int(2))] => {},
//         _ => panic!("Unexpected frame content: {:?}", top),
//     }

//     // Drop top frame, should restore the previous one
//     stack.flush(1);

//     let after_drop = stack.peek_many(2).expect("Expected frame after drop");
//     assert_eq!(after_drop, &[a, b]);

//     // Drop again to empty the stack
//     stack.flush(2);

//     // Should now be empty
//     assert!(stack.peek().is_none());
// }

// #[test]
// fn test_value_stack_call_split() {
//     let mut storage = make_storage::<Value, 10>();
//     let mut stack = ValueStack::new(StackRef::from_slice(&mut storage));

//     let a = Value::Int(10);
//     let b = Value::Int(20);

//     // Push initial frame
//     assert!(stack.push_slice(&[a, b]).is_ok());

//     // Now invoke call_split to construct a new frame that depends on the current one
//     let result = stack.call_split(|input, out_stack| {
//         assert_eq!(input, &[a, b]); // left is full frame (excluding the Frame marker)
//         let cons = Value::Cons(&input[0], &input[1]);
//         out_stack.push(cons)
//         .map_err(|_|())
//     });

//     assert!(result.is_ok());
//     assert_eq!(stack.peek_all(),&[a,b,Value::Cons(&a,&b)]);

//     // Verify the top frame is the one pushed inside `call_split`
//     let top = stack.peek().expect("Expected a top frame after call_split");
//     match top {
//         Value::Cons(Value::Int(10), Value::Int(20)) => {},
//         _ => panic!("Unexpected top frame: {:?}", top),
//     }

//     // Cleanup both frames
//     stack.flush(3);

//     assert!(stack.peek().is_none());

// }

// #[test]
// fn test_value_stack_call_split_drop() {
//     use Value::*;

//     // backing store that is comfortably large
//     let mut storage = make_storage::<Value, 16>();
//     let mut stack   = ValueStack::new(StackRef::from_slice(&mut storage));

//     // ── frame #0 ──────────────────────────────────────────────
//     let base = Int(9);
//     stack.push(base).unwrap();
//     // stack:  Int(9)  Frame(1)

//     // ── frame #1 (will be “dropped” inside call_split_drop) ──
//     let x = Int(1);
//     let y = Int(2);
//     stack.push_slice(&[x, y]).unwrap();
//     // stack:  Int(9) Frame(1)  Int(1) Int(2) Frame(2)

//     // ---------------------------------------------------------
//     stack
//         .call_split_drop(2,|rest, frame, inner| {
//             // `rest` should be the whole prefix before the frame-to-drop
//             assert_eq!(rest, &[base]);
//             assert_eq!(frame, &[x, y]);

//             // create a new value that *only* references `rest`,
//             // never the soon-to-be-flushed `frame`.
//             let cons   = Cons(&rest[0], &rest[0]);

//             // push a replacement frame
//             inner
//                 .push_n([cons,frame[1]])
//                 .map_err(|_| ())
//         })
//         .unwrap();

//     // expected final layout:
//     //   Int(9)  Frame(1)           ← original lower frame
//     //   Cons(&Int(9), &Int(42))    ← replacement upper frame
//     //   Frame(1)
//     let expected_cons = Value::Cons(&base,&base);
//     assert_eq!(
//         stack.peek_all(),
//         &[base, expected_cons,y]
//     );

// }

// #[test]
// fn test_overlapping_copy_call_split_drop() {
//     use Value::*;

//     let mut storage = make_storage::<Value, 6>();
//     let mut stack   = ValueStack::new(StackRef::from_slice(&mut storage));

//     // lower frame (size 1)
//     stack.push_slice(&[Int(0)]).unwrap();                // ... Frame(1)

//     // upper frame (size 2) – will be dropped
//     stack.push_slice(&[Int(1), Int(2)]).unwrap();        // ... Int Int Frame(2)

//     // replacement frame is *larger* than the one we are about to flush,
//     // so the source slice lives at higher addresses than the destination.
//     // If the implementation copies with `copy_nonoverlapping`, that's UB.
//     stack
//         .call_split_drop(2,|_rest, _frame, inner| {
//             inner
//                 .push_n([
//                     Int(10),
//                     Int(11),
//                     Int(12),          // <- 3 elements, not 2
//                 ])
//                 .map_err(|_| ())
//         })
//         .unwrap();
// }