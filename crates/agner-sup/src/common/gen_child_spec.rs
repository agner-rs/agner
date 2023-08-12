mod args_call;
mod args_clone;
mod args_unique;
mod gen_child_spec_impl;
mod traits;

#[cfg(test)]
mod tests;

use std::marker::PhantomData;

#[cfg(feature = "reg")]
use agner_reg::RegTx;
pub use traits::{CreateArgs, CreateChild};

use crate::common::init_type::InitType;

/// A base for both [uniform](crate::uniform::UniformChildSpec) and
/// [mixed](crate::mixed::MixedChildSpec) child specifications.
pub struct GenChildSpec<B, A, M, X> {
    behaviour: B,
    create_args: A,
    message: PhantomData<M>,
    init_type: InitType,

    #[cfg(feature = "reg")]
    reg_tx: Option<RegTx>,

    ext: X,
}
