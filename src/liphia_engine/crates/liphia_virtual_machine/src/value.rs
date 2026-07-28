// liphia_virtual_machine/src/value.rs

use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

#[derive(Debug, Clone)]
pub enum Value {
    Int(i64),
    Float(f64),
    Bool(bool),
    Str(Rc<String>),
    List(Rc<RefCell<Vec<Value>>>),
    // Associative list — Vec<(key, value)> instead of HashMap, because
    // Value contains Float and does not implement Eq/Hash.
    Map(Rc<RefCell<Vec<(Value, Value)>>>),
    EnumVariant { enum_name: Rc<String>, variant: Rc<String> },
    Null,
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Int(a),   Value::Int(b))   => a == b,
            (Value::Float(a), Value::Float(b)) => a == b,
            (Value::Bool(a),  Value::Bool(b))  => a == b,
            (Value::Str(a),   Value::Str(b))   => a == b,
            (Value::Null,     Value::Null)      => true,
            (Value::List(a),  Value::List(b))  => {
                Rc::ptr_eq(a, b) || *a.borrow() == *b.borrow()
            }
            (Value::Map(a),   Value::Map(b))   => {
                Rc::ptr_eq(a, b) || *a.borrow() == *b.borrow()
            }
            (Value::EnumVariant { enum_name: e1, variant: v1 },
             Value::EnumVariant { enum_name: e2, variant: v2 }) => e1 == e2 && v1 == v2,
            _ => false,
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Int(v)    => write!(f, "{}", v),
            Value::Float(v)  => write!(f, "{}", v),
            Value::Bool(v)   => write!(f, "{}", if *v { "true" } else { "false" }),
            Value::Str(v)    => write!(f, "{}", v),
            Value::Null      => write!(f, "null"),
            Value::List(v)   => {
                let items = v.borrow();
                write!(f, "[")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", item)?;
                }
                write!(f, "]")
            }
            Value::Map(v) => {
                let items = v.borrow();
                write!(f, "{{")?;
                for (i, (k, val)) in items.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}: {}", k, val)?;
                }
                write!(f, "}}")
            }
            Value::EnumVariant { enum_name, variant } => {
                write!(f, "{}.{}", enum_name, variant)
            }
        }
    }
}
