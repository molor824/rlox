use std::{cell::RefCell, rc::Rc};

use crate::{
    error::ErrorKind,
    interpreter::{
        value::{Object, Value},
        Interpreter,
    },
};

pub fn print(interpreter: &mut Interpreter) -> Result<Value, ErrorKind> {
    let args = interpreter.get_local(0).try_array()?;
    let mut first = true;
    for arg in args.borrow().iter() {
        if !first {
            print!(" ");
        }
        first = false;
        print!("{}", arg);
    }
    Ok(Value::Nil)
}
pub fn println(interpreter: &mut Interpreter) -> Result<Value, ErrorKind> {
    print(interpreter)?;
    println!();
    Ok(Value::Nil)
}
pub fn array(interpreter: &mut Interpreter) -> Result<Value, ErrorKind> {
    let iter = interpreter.get_local(0).try_iterator()?;
    let array = Value::Array(Rc::new(RefCell::new(iter.collect::<Vec<_>>())));
    Ok(array)
}
pub fn object(interpreter: &mut Interpreter) -> Result<Value, ErrorKind> {
    let iter = interpreter.get_local(0).try_iterator()?.map(|value| {
        value.try_array().map(|array| {
            (
                array.borrow().get(0).cloned().unwrap_or_default(),
                array.borrow().get(1).cloned().unwrap_or_default(),
            )
        })
    });
    let obj = Value::Object(Rc::new(RefCell::new(Object::new(
        iter.collect::<Result<_, _>>()?,
    )?)));
    Ok(obj)
}
pub fn length(interpreter: &mut Interpreter) -> Result<Value, ErrorKind> {
    let value = interpreter.get_local(0);
    match value {
        Value::Array(arr) => Ok(Value::Number(arr.borrow().len() as f64)),
        Value::Object(obj) => Ok(Value::Number(obj.borrow().map.len() as f64)),
        Value::String(str) => Ok(Value::Number(str.as_str().len() as f64)),
        v => Err(ErrorKind::RuntimeError(format!(
            "Value of type `{}` does not have length definition",
            v.type_str()
        ))),
    }
}

pub const GLOBALS: [(
    &str,
    usize,
    bool,
    fn(&mut Interpreter) -> Result<Value, ErrorKind>,
); 5] = [
    ("print", 0, true, print),
    ("println", 0, true, println),
    ("array", 1, false, array),
    ("object", 1, false, object),
    ("len", 1, false, length),
];
