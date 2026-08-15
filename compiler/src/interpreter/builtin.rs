use std::{cell::RefCell, fmt::Write, mem::replace, rc::Rc};

use rustc_hash::FxHashMap;

use crate::{
    error::ErrorKind,
    interpreter::{
        string::ValueStr,
        value::{Function, Value},
        Interpreter,
    },
};

fn print(interpreter: &mut Interpreter) -> Result<Value, ErrorKind> {
    let args = interpreter.get_local(0).try_array().unwrap();
    for arg in args.borrow().iter() {
        print!("{}", arg);
    }
    Ok(Value::Nil)
}
fn println(interpreter: &mut Interpreter) -> Result<Value, ErrorKind> {
    print(interpreter)?;
    println!();
    Ok(Value::Nil)
}
fn str(interpreter: &mut Interpreter) -> Result<Value, ErrorKind> {
    let args = interpreter.get_local(0).try_array().unwrap();
    let mut string = String::with_capacity(args.borrow().len());
    for arg in args.borrow().iter() {
        write!(string, "{}", arg).unwrap();
    }
    Ok(Value::String(ValueStr::from(string.as_str())))
}
fn length(interpreter: &mut Interpreter) -> Result<Value, ErrorKind> {
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
fn set_base_obj(interpreter: &mut Interpreter) -> Result<Value, ErrorKind> {
    let src = interpreter.get_local(0).try_object()?;
    let base = interpreter.get_local(1).try_object()?;
    src.borrow_mut().base_obj = Some(base);
    Ok(Value::Object(src))
}
fn get_base_obj(interpreter: &mut Interpreter) -> Result<Value, ErrorKind> {
    interpreter
        .get_local(0)
        .try_object()
        .map(|obj| match obj.borrow().base_obj.clone() {
            Some(base) => Value::Object(base),
            None => Value::Nil,
        })
}
fn sqrt(interpreter: &mut Interpreter) -> Result<Value, ErrorKind> {
    interpreter
        .get_local(0)
        .try_num()
        .map(|n| Value::Number(n.sqrt()))
}
fn iter(interpreter: &mut Interpreter) -> Result<Value, ErrorKind> {
    let iterable = interpreter.get_local(0);
    let iter_fn = match iterable {
        Value::Array(arr) => {
            let len = arr.borrow().len();
            let mut idx = 0;
            Rc::new(Interpreter::create_builtin_function(0, false, move |_| {
                if idx < len {
                    let v = arr.borrow().get(idx).cloned().unwrap_or_default();
                    idx += 1;
                    Ok(v)
                } else {
                    Ok(Value::Nil)
                }
            }))
        }
        Value::Object(obj) => {
            let mut item_iter = obj
                .borrow()
                .map
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect::<Vec<_>>()
                .into_iter();
            Rc::new(Interpreter::create_builtin_function(
                0,
                false,
                move |_| match item_iter.next() {
                    Some((k, v)) => Ok(Value::Array(Rc::new(RefCell::new(vec![k, v])))),
                    None => Ok(Value::Nil),
                },
            ))
        }
        Value::String(str) => {
            let mut offset = 0;
            Rc::new(Interpreter::create_builtin_function(
                0,
                false,
                move |_| match str.as_str()[offset..].chars().next() {
                    Some(ch) => {
                        let prev_offset = offset;
                        offset += ch.len_utf8();
                        Ok(Value::String(str.as_str()[prev_offset..offset].into()))
                    }
                    None => Ok(Value::Nil),
                },
            ))
        }
        Value::Function(func) => func,
        t => return Err(ErrorKind::UniterableType(t.type_str())),
    };
    Ok(Value::Function(iter_fn))
}
fn range(interpreter: &mut Interpreter) -> Result<Value, ErrorKind> {
    let mut start = interpreter.get_local(0).try_num().ok().unwrap_or_default();
    let end = match interpreter.get_local(1).try_num().ok() {
        Some(n) => n,
        None => replace(&mut start, 0.0),
    };
    let step = interpreter
        .get_local(2)
        .try_num()
        .ok()
        .unwrap_or(if start <= end { 1.0 } else { -1.0 });

    let iter_fn = move |_: &mut Interpreter| -> Result<Value, ErrorKind> {
        if start.abs() >= end.abs() {
            Ok(Value::Nil)
        } else {
            let v = Value::Number(start);
            start += step;
            Ok(v)
        }
    };
    let iter = Value::Function(Rc::new(Interpreter::create_builtin_function(
        0, false, iter_fn,
    )));
    Ok(iter)
}
thread_local! {
    static ITER_FN: Rc<Function> = GLOBALS
        .with(|globals| globals.get(&ValueStr::interned("iter")).unwrap().clone());
}
fn map(interpreter: &mut Interpreter) -> Result<Value, ErrorKind> {
    let iterable = interpreter.get_local(0);
    let functor = interpreter.get_local(1).try_function()?;
    let iterator = interpreter
        .call_function_args(ITER_FN.with(|iter| iter.clone()), [iterable])?
        .try_function()?;

    let mapped_iter_fn = Interpreter::create_builtin_function(0, false, move |interpreter| {
        let item = interpreter.call_function_args(iterator.clone(), [])?;
        if item == Value::Nil {
            return Ok(item);
        }
        interpreter.call_function_args(functor.clone(), [item])
    });

    Ok(Value::Function(Rc::new(mapped_iter_fn)))
}
fn filter(interpreter: &mut Interpreter) -> Result<Value, ErrorKind> {
    let iterable = interpreter.get_local(0);
    let predicate = interpreter.get_local(1).try_function()?;
    let iterator = interpreter
        .call_function_args(ITER_FN.with(|f| f.clone()), [iterable])?
        .try_function()?;

    let filtered_iter_fn =
        Interpreter::create_builtin_function(0, false, move |interpreter| loop {
            let item = interpreter.call_function_args(iterator.clone(), [])?;
            if item == Value::Nil {
                return Ok(item);
            }
            let filter = interpreter.call_function_args(predicate.clone(), [item.clone()])?;

            if filter.as_bool() {
                return Ok(item);
            }
        });

    Ok(Value::Function(Rc::new(filtered_iter_fn)))
}

thread_local! {
    pub static GLOBALS: FxHashMap<ValueStr, Rc<Function>> = [
        ("print", 0, true, print as fn(&mut Interpreter) -> Result<Value, ErrorKind>),
        ("println", 0, true, println),
        ("len", 1, false, length),
        ("setbase", 2, false, set_base_obj),
        ("getbase", 1, false, get_base_obj),
        ("sqrt", 1, false, sqrt),
        ("iter", 1, false, iter),
        ("range", 3, false, range),
        ("str", 0, true, str),
        ("map", 2, false, map),
        ("filter", 2, false, filter),
    ].into_iter().map(|(name, arity, variadic, ptr)| (
        ValueStr::interned(name),
        Rc::new(Interpreter::create_builtin_function(arity, variadic, ptr))
    )).collect();
}
