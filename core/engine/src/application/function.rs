//! [`EngineFunction`] — the auxiliary trait PRD-002:49-51 names in the bound of
//! `register_fn` but never defines.
//!
//! It adapts an ordinary Rust closure into something an engine can call: it
//! takes a slice of [`EngineValue`] arguments and returns one [`EngineValue`],
//! mapping every conversion failure to [`EngineError`]. Blanket impls cover
//! closures of arity 0 through 4 whose parameters are [`FromEngineValue`] and
//! whose result is [`IntoEngineValue`]. Fallible or variadic native functions
//! register through
//! [`ExecutionContext::register_native_fn`][crate::ExecutionContext::register_native_fn]
//! directly; broader arity support can be added without changing this contract.

use crate::application::conversion::{FromEngineValue, IntoEngineValue};
use crate::domain::error::EngineError;
use crate::domain::value::EngineValue;

/// The number of arguments a native function accepts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Arity(usize);

impl Arity {
    #[must_use]
    pub const fn exact(count: usize) -> Self {
        Self(count)
    }

    #[must_use]
    pub const fn count(self) -> usize {
        self.0
    }
}

/// A Rust callable exposed to scripts under a name. `Args` and `Ret` are marker
/// type parameters that let the blanket impls below stay non-overlapping.
pub trait EngineFunction<Args, Ret>: Send + Sync + 'static {
    /// Invoke with already-marshalled arguments.
    fn invoke(&self, arguments: &[EngineValue]) -> Result<EngineValue, EngineError>;

    /// How many arguments [`invoke`](Self::invoke) expects.
    fn arity(&self) -> Arity;
}

fn check_arity(arguments: &[EngineValue], expected: usize) -> Result<(), EngineError> {
    if arguments.len() == expected {
        return Ok(());
    }
    Err(EngineError::binding(format!(
        "expected {expected} argument(s), received {}",
        arguments.len()
    )))
}

fn argument<T: FromEngineValue>(arguments: &[EngineValue], index: usize) -> Result<T, EngineError> {
    let slot = arguments
        .get(index)
        .ok_or_else(|| EngineError::binding(format!("missing argument {index}")))?;
    T::from_engine_value(slot.clone())
}

impl<Function, Ret> EngineFunction<(), Ret> for Function
where
    Function: Fn() -> Ret + Send + Sync + 'static,
    Ret: IntoEngineValue + 'static,
{
    fn invoke(&self, arguments: &[EngineValue]) -> Result<EngineValue, EngineError> {
        check_arity(arguments, 0)?;
        Ok((self)().into_engine_value())
    }

    fn arity(&self) -> Arity {
        Arity::exact(0)
    }
}

impl<Function, A0, Ret> EngineFunction<(A0,), Ret> for Function
where
    Function: Fn(A0) -> Ret + Send + Sync + 'static,
    A0: FromEngineValue + 'static,
    Ret: IntoEngineValue + 'static,
{
    fn invoke(&self, arguments: &[EngineValue]) -> Result<EngineValue, EngineError> {
        check_arity(arguments, 1)?;
        let a0 = argument::<A0>(arguments, 0)?;
        Ok((self)(a0).into_engine_value())
    }

    fn arity(&self) -> Arity {
        Arity::exact(1)
    }
}

impl<Function, A0, A1, Ret> EngineFunction<(A0, A1), Ret> for Function
where
    Function: Fn(A0, A1) -> Ret + Send + Sync + 'static,
    A0: FromEngineValue + 'static,
    A1: FromEngineValue + 'static,
    Ret: IntoEngineValue + 'static,
{
    fn invoke(&self, arguments: &[EngineValue]) -> Result<EngineValue, EngineError> {
        check_arity(arguments, 2)?;
        let a0 = argument::<A0>(arguments, 0)?;
        let a1 = argument::<A1>(arguments, 1)?;
        Ok((self)(a0, a1).into_engine_value())
    }

    fn arity(&self) -> Arity {
        Arity::exact(2)
    }
}

impl<Function, A0, A1, A2, Ret> EngineFunction<(A0, A1, A2), Ret> for Function
where
    Function: Fn(A0, A1, A2) -> Ret + Send + Sync + 'static,
    A0: FromEngineValue + 'static,
    A1: FromEngineValue + 'static,
    A2: FromEngineValue + 'static,
    Ret: IntoEngineValue + 'static,
{
    fn invoke(&self, arguments: &[EngineValue]) -> Result<EngineValue, EngineError> {
        check_arity(arguments, 3)?;
        let a0 = argument::<A0>(arguments, 0)?;
        let a1 = argument::<A1>(arguments, 1)?;
        let a2 = argument::<A2>(arguments, 2)?;
        Ok((self)(a0, a1, a2).into_engine_value())
    }

    fn arity(&self) -> Arity {
        Arity::exact(3)
    }
}

impl<Function, A0, A1, A2, A3, Ret> EngineFunction<(A0, A1, A2, A3), Ret> for Function
where
    Function: Fn(A0, A1, A2, A3) -> Ret + Send + Sync + 'static,
    A0: FromEngineValue + 'static,
    A1: FromEngineValue + 'static,
    A2: FromEngineValue + 'static,
    A3: FromEngineValue + 'static,
    Ret: IntoEngineValue + 'static,
{
    fn invoke(&self, arguments: &[EngineValue]) -> Result<EngineValue, EngineError> {
        check_arity(arguments, 4)?;
        let a0 = argument::<A0>(arguments, 0)?;
        let a1 = argument::<A1>(arguments, 1)?;
        let a2 = argument::<A2>(arguments, 2)?;
        let a3 = argument::<A3>(arguments, 3)?;
        Ok((self)(a0, a1, a2, a3).into_engine_value())
    }

    fn arity(&self) -> Arity {
        Arity::exact(4)
    }
}
