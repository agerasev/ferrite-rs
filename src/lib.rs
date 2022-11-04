mod raw;

pub mod misc;
pub mod variable;

pub use raw::export;
pub use variable::{
    AnyVariable, Downcast, ReadArrayVariable, ReadVariable, VariableType, WriteArrayVariable,
    WriteVariable,
};

use std::collections::HashMap;

#[macro_export]
macro_rules! entry_point {
    (
        $(#[$fn_meta:meta])*
        $fn_vis:vis fn $fn_name:ident(mut $arg_name:ident : $arg_type:ty $(,)?)
        $fn_body:block
    ) => (
        $(#[$fn_meta])*
        $fn_vis fn $fn_name(mut $arg_name : $arg_type)
        $fn_body

        #[no_mangle]
        pub extern "Rust" fn ferrite_app_main(ctx: $crate::Context) {
            $fn_name(ctx)
        }
    );
}

pub type Registry = HashMap<String, AnyVariable>;

pub struct Context {
    pub registry: Registry,
}
