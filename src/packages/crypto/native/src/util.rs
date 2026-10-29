// packages/crypto/native/src/util.rs
//
// Argument helpers shared by the natives of this package. Messages follow
// the core natives ("name(): ...") so errors read the same everywhere.

use std::rc::Rc;

use liphia_virtual_machine::value::Value;
use liphia_virtual_machine::vm::{VmError, VmResult};

pub fn expect_args(name: &str, args: &[Value], expected: usize) -> VmResult<()> {
    if args.len() != expected {
        return Err(VmError::new(format!(
            "{}() expects {} argument(s), got {}",
            name,
            expected,
            args.len()
        )));
    }
    Ok(())
}

// Borrows the text of a str argument. No copy: inputs can be large and the
// Value already owns the string.
pub fn str_ref<'a>(value: &'a Value, name: &str) -> VmResult<&'a str> {
    match value {
        Value::Str(s) => Ok(s.as_str()),
        _ => Err(VmError::new(format!("{}(): argument must be str", name))),
    }
}

pub fn str_value(text: impl Into<String>) -> Value {
    Value::Str(Rc::new(text.into()))
}
