mod args_call;
mod args_clone;
mod args_unique;
mod gen_child_spec_impl;
mod traits;

#[cfg(test)]
mod tests;

use std::marker::PhantomData;

use agner_reg::Service;
pub use traits::{CreateArgs, CreateChild};

use crate::common::init_type::InitType;

pub struct GenChildSpec<B, A, M, X> {
    behaviour: B,
    create_args: A,
    message: PhantomData<M>,
    init_type: InitType,

    #[cfg(feature = "reg")]
    service: Option<Service>,

    ext: X,
}
