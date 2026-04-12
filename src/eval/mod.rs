pub(crate) mod functions;
mod handle;
pub mod input;
pub mod numeric;
pub(crate) mod scalar;

pub use handle::{EvalHandle, EvalIter, eval};
pub use input::EvalInput;
pub use numeric::NumericResult;
