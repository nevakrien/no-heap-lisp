use crate::stack::make_storage;
use crate::stack::StackRef;

#[derive(Debug,Clone,Copy)]
pub enum Value<'a>{
	Frame(usize),

	Nil,
	False,
	True,
	Int(i64),
	Cons(&'a Value<'a>,&'a Value<'a>),
}

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

	pub fn push_dependent<F>(&mut self,f:F) ->Result<(),()>
	where F:for<'b> FnOnce(&'b [Value<'v>])->&'b [Value<'v>]{
		let num_wrote = {
			let (left,right) = self.0.split();
			let vals = f(left);
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

// pub unsafe fn match_stack_lifetime<'v>(_stack:*const StackRef<Value<'v>>,values:&[Value]) -> &'v [Value<'v>]{
// 	unsafe{
// 		let p = values as *const _ as *const [Value<'v>];
// 		&*p
// 	}
// }

// #[cfg(test)]
// fn push_pop_cons<'a,'v : 'a>(stack:&mut StackRef<'a,Value<'v>>){
//     let p = stack as *const _;
//     {
// 	    stack.push(Value::Nil).unwrap();
// 	    stack.push(Value::Nil).unwrap();

// 	    let (left,mut right) = stack.split();
// 	    let left : &'v [Value<'v>]= unsafe {match_stack_lifetime(p,left)};
// 	    right.push(Value::Cons(&left[0],&left[1])).unwrap();

// 	}
// 	unsafe{stack.advance(1);}

// 	let arr = stack.pop_n::<3>().unwrap();
// 	stack.push_n::<3>([Value::Nil;3]).unwrap();
// 	assert!(matches!(arr[2],Value::Cons(_,&Value::Nil)));
// 	assert!(stack.pop().is_none());
// }
// #[test]
// fn test_value_lifetime(){
// 	let mut storage = make_storage::<Value, 6>();
//     let mut stack = StackRef::from_slice(&mut storage);
//     push_pop_cons(&mut stack); 
// }