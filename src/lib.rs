mod downcast;
mod import;

pub mod atomic;
pub mod export;
pub mod registry;
pub mod typed;
pub mod variable;

pub use downcast::Downcast;
pub use registry::Registry;
pub use typed::{FlatVec, TypedVariable};
pub use variable::{Info, Variable};

pub struct Context {
    pub registry: Registry,
}

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
