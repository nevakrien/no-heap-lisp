use crate::stack::make_storage;
use crate::stack::StackRef;

#[derive(Debug,Clone,Copy)]
pub enum Value<'a>{
	Nil,
	False,
	True,
	Int(i64),
	Cons(&'a Value<'a>,&'a Value<'a>)
}

// #[test]
// fn test_value_lifetime(){
// 	let mut storage = make_storage::<Value, 6>();
//     let mut stack = StackRef::from_slice(&mut storage);
//     {
// 	    stack.push(Value::Nil).unwrap();
// 	    stack.push(Value::Nil).unwrap();

// 	    let (left,mut right) = stack.split();
// 	    let [a,b] = left.peek_n::<2>().unwrap();
// 	    right.push(Value::Cons(a,b)).unwrap();


// 	}

// 	stack.pop_n::<3>().unwrap();
// 	assert!(stack.pop().is_none());
// }