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
        let arr = value.try_array()?;
        let arr = arr.borrow();
        Ok((
            arr.get(0).cloned().unwrap_or_default().try_str()?,
            arr.get(1).cloned().unwrap_or_default(),
        )) as Result<_, ErrorKind>
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
pub fn set_base_obj(interpreter: &mut Interpreter) -> Result<Value, ErrorKind> {
    let src = interpreter.get_local(0).try_object()?;
    let base = interpreter.get_local(1).try_object()?;
    src.borrow_mut().base_obj = Some(base);
    Ok(Value::Object(src))
}
pub fn get_base_obj(interpreter: &mut Interpreter) -> Result<Value, ErrorKind> {
    interpreter
        .get_local(0)
        .try_object()
        .map(|obj| match obj.borrow().base_obj.clone() {
            Some(base) => Value::Object(base),
            None => Value::Nil,
        })
}
pub fn sqrt(interpreter: &mut Interpreter) -> Result<Value, ErrorKind> {
    interpreter
        .get_local(0)
        .try_num()
        .map(|n| Value::Number(n.sqrt()))
}

pub const GLOBALS: [(
    &str,
    usize,
    bool,
    fn(&mut Interpreter) -> Result<Value, ErrorKind>,
); 8] = [
    ("print", 0, true, print),
    ("println", 0, true, println),
    ("array", 1, false, array),
    ("object", 1, false, object),
    ("len", 1, false, length),
    ("setbase", 2, false, set_base_obj),
    ("getbase", 1, false, get_base_obj),
    ("sqrt", 1, false, sqrt),
];
