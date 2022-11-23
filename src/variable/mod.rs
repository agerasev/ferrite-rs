pub mod any;
pub mod array;
pub mod atomic;
pub mod registry;
pub mod scalar;
pub mod sync;

pub use any::{AnyVariable, Downcast, Var};
pub use array::ArrayVariable;
pub use registry::Registry;
pub use scalar::Variable;
pub use sync::VarSync;
