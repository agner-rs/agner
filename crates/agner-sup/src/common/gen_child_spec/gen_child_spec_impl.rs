use std::fmt;

use agner_actors::{Actor, ActorID, System};

#[cfg(feature = "reg")]
use agner_reg::RegTx;
use futures::TryFutureExt;

use crate::common::gen_child_spec::args_call::{args_call0, args_call1, ArgsCallFn0, ArgsCallFn1};
use crate::common::gen_child_spec::args_clone::{args_clone, ArgsClone};
use crate::common::gen_child_spec::args_unique::{args_unique, ArgsUnique};
use crate::common::gen_child_spec::traits::{CreateArgs, CreateChild};
use crate::common::gen_child_spec::GenChildSpec;
use crate::common::start_child::start_child;
use crate::common::InitType;

impl GenChildSpec<(), (), (), ()> {
    pub fn new() -> Self {
        Default::default()
    }
}

impl Default for GenChildSpec<(), (), (), ()> {
    fn default() -> Self {
        GenChildSpec {
            behaviour: (),
            create_args: (),
            message: Default::default(),
            init_type: InitType::NoAck,

            #[cfg(feature = "reg")]
            reg_tx: None,

            ext: (),
        }
    }
}

impl<X> GenChildSpec<(), (), (), X> {
    pub fn from_ext(ext: X) -> Self {
        GenChildSpec {
            behaviour: (),
            create_args: (),
            message: Default::default(),
            init_type: InitType::NoAck,

            #[cfg(feature = "reg")]
            reg_tx: None,

            ext,
        }
    }
}

impl<B, A, M, X> GenChildSpec<B, A, M, X> {
    pub fn ext(&self) -> &X {
        &self.ext
    }
    pub fn ext_mut(&mut self) -> &mut X {
        &mut self.ext
    }
}

impl<A, M, X> GenChildSpec<(), A, M, X> {
    pub fn behaviour<B>(self, behaviour: B) -> GenChildSpec<B, A, M, X> {
        GenChildSpec {
            behaviour,
            create_args: self.create_args,
            message: Default::default(),
            init_type: self.init_type,

            #[cfg(feature = "reg")]
            reg_tx: self.reg_tx,

            ext: self.ext,
        }
    }
}

impl<B, OldA, OldM, X> GenChildSpec<B, OldA, OldM, X> {
    pub fn args_clone<A, M>(self, args: A) -> GenChildSpec<B, ArgsClone<A>, M, X>
    where
        B: for<'a> Actor<'a, A, M>,
        ArgsClone<A>: CreateArgs<Input = (), Output = A>,
    {
        let create_args = args_clone(args);
        GenChildSpec {
            behaviour: self.behaviour,
            create_args,
            message: Default::default(),
            init_type: self.init_type,

            #[cfg(feature = "reg")]
            reg_tx: self.reg_tx,

            ext: self.ext,
        }
    }

    pub fn args_unique<A, M>(self, args: A) -> GenChildSpec<B, ArgsUnique<A>, M, X>
    where
        B: for<'a> Actor<'a, Option<A>, M>,
        ArgsUnique<A>: CreateArgs<Input = (), Output = Option<A>>,
    {
        let create_args = args_unique(args);
        GenChildSpec {
            behaviour: self.behaviour,
            create_args,
            message: Default::default(),
            init_type: self.init_type,

            #[cfg(feature = "reg")]
            reg_tx: self.reg_tx,

            ext: self.ext,
        }
    }

    pub fn args_call0<F, Out, M>(self, make_args: F) -> GenChildSpec<B, ArgsCallFn0<F, Out>, M, X>
    where
        B: for<'a> Actor<'a, Out, M>,
        ArgsCallFn0<F, Out>: CreateArgs<Input = (), Output = Out>,
    {
        let create_args = args_call0(make_args);
        GenChildSpec {
            behaviour: self.behaviour,
            create_args,
            message: Default::default(),
            init_type: self.init_type,

            #[cfg(feature = "reg")]
            reg_tx: self.reg_tx,

            ext: self.ext,
        }
    }

    pub fn args_call1<F, In, Out, M>(
        self,
        make_args: F,
    ) -> GenChildSpec<B, ArgsCallFn1<F, In, Out>, M, X>
    where
        B: for<'a> Actor<'a, Out, M>,
        ArgsCallFn1<F, In, Out>: CreateArgs<Input = In, Output = Out>,
    {
        let create_args = args_call1(make_args);
        GenChildSpec {
            behaviour: self.behaviour,
            create_args,
            message: Default::default(),
            init_type: self.init_type,

            #[cfg(feature = "reg")]
            reg_tx: self.reg_tx,

            ext: self.ext,
        }
    }
}

impl<B, A, M, X> GenChildSpec<B, A, M, X> {
    /// Specify child's [init-type](crate::common::InitType)
    pub fn init_type<IT>(self, init_type: IT) -> Self
    where
        IT: Into<InitType>,
    {
        let init_type = init_type.into();
        Self { init_type, ..self }
    }
}

#[cfg(feature = "reg")]
impl<B, A, M, X> GenChildSpec<B, A, M, X> {
    /// [Register the actor](agner_reg::Service) when it starts
    pub fn register(self, reg_tx: RegTx) -> Self {
        let reg_tx = Some(reg_tx);
        Self { reg_tx, ..self }
    }
}

impl<B, A, M, X> CreateChild for GenChildSpec<B, A, M, X>
where
    B: for<'a> Actor<'a, A::Output, M>,
    A: CreateArgs,
    B: Clone,
    M: Unpin + Send + 'static,
    A::Output: Send + 'static,
{
    type Args = A::Input;

    fn create_child(
        &mut self,
        system: &System,
        sup_id: ActorID,
        args: Self::Args,
    ) -> crate::common::StaticBoxedFuture<
        Result<agner_actors::ActorID, crate::common::StartChildError>,
    > {
        let system = system.to_owned();
        let args = self.create_args.create_args(args);
        let behaviour = self.behaviour.to_owned();
        let init_type = self.init_type;

        #[cfg(feature = "reg")]
        let registered_service = self.reg_tx.to_owned();

        let start_child_fut = start_child(system.to_owned(), sup_id, behaviour, args, init_type)
            .and_then(move |child_id| async move {
                #[cfg(feature = "reg")]
                if let Some(service) = registered_service {
                    let reg_guard = service.register(child_id);
                    system.put_data(child_id, reg_guard).await;
                }

                Ok(child_id)
            });

        Box::pin(start_child_fut)
    }
}

impl<B, A, M, X> fmt::Debug for GenChildSpec<B, A, M, X>
where
    A: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CreateChildImpl")
            .field("behaviour", &std::any::type_name::<B>())
            .field("create_args", &self.create_args)
            .field("init_type", &self.init_type)
            .finish()
    }
}

impl<B, A, M, X> Clone for GenChildSpec<B, A, M, X>
where
    B: Clone,
    A: Clone,
    X: Clone,
{
    fn clone(&self) -> Self {
        Self {
            behaviour: self.behaviour.clone(),
            create_args: self.create_args.clone(),
            message: Default::default(),
            init_type: self.init_type,

            #[cfg(feature = "reg")]
            reg_tx: self.reg_tx.clone(),

            ext: self.ext.clone(),
        }
    }
}
