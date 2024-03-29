//! agner — ~~processes~~ actors.
//! =====
//!
//! Note: Right now this is a research project, i.e. it is possible that the API will undergo
//! incompatible changes within the version 0.3.x.
//!
//! As it has been stated, agner is inspired by Erlang/OTP, so all similarities to the actual
//! frameworks, supported or obsolete, are purely intentional. :)
//!
//!
//! # [Actors](crate::actors)
//!
//! An [actor](https://en.wikipedia.org/wiki/Actor_model) is an activity with the following properties:
//! - runs in parallel (implemented as a [`Future`](std::future::Future));
//! - has a handle ([`ActorID`](crate::actors::ActorID));
//! - can receive messages;
//! - when terminates — yields an exit reason ([`Exit`](crate::actors::Exit));
//! - any two actors can be [linked](crate::actors::Context::link) with each other:
//!     - if one of the linked actors exits with a reason other than
//!       [`Exit::normal()`](crate::actors::Exit::normal()) — the other receives an exit-signal;
//!     - if the process receiving an exit-signal does not ["trap
//!       exits"](crate::actors::Context::trap_exit), it will also be terminated.
//!
//! ## Implementing an Actor
//! The actor's behaviour is defined by:
//! - the type of its argument;
//! - the type of the message it accepts;
//! - the behaviour function.
//!
//! In order to implement an actor one should define an async function that
//! - returns a value for which the trait [`Into<Exit>`](crate::actors::Exit) is
//! defined
//! - and accepts two arguments:
//!     - a mutable reference to [`Context<Message>`](crate::actors::Context);
//!     - `Argument`.
//!
//! Example:
//! ```
//! use agner::actors::{Context, Exit, Never};
//!
//! async fn shutdown_after_six_messages(context: &mut Context<String>, actor_name: String) {
//!     for i in 0..6 {
//!         let message = context.next_message().await;
//!         eprintln!("Actor {:?} received {:?}", actor_name, message);
//!     }
//! }
//! ```
//!
//! ## Spawning an Actor
//!
//! Actors cannot run on their own, they need an [actor system](crate::actors::System) to be spawned
//! in. This is necessary to avoid having a global state imposed by mere usage of the library.
//! A [`System`](crate::actors::System) is a scope within which the actors run.
//!
//! Example:
//! ```
//! use agner::actors::{System, Context, Exit, Never};
//!
//! async fn a_dummy(context: &mut Context<Option<String>>, actor_name: String) {
//!     eprintln!("{}: hi!", actor_name);
//!
//!     while let Some(message) = context.next_message().await {
//!         eprintln!("{}: received {:?}", actor_name, message);
//!     }
//!
//!     eprintln!("{}: bye!", actor_name);
//! }
//!
//! #[tokio::main]
//! async fn main() {
//!     // create a system with default configuration
//!     let system = System::new(Default::default());
//!
//!     let actor_id = system.spawn(
//!         a_dummy,
//!         "the-dummy".to_owned(),
//!         Default::default())
//!             .await.expect("Failed to spawn an actor");
//!
//!     system.send(actor_id, Some("one".to_owned())).await;
//!     system.send(actor_id, Some("two".to_owned())).await;
//!     system.send(actor_id, Some("three".to_owned())).await;
//!     system.send(actor_id, Option::<String>::None).await;
//!
//!     let exit_reason = system.wait(actor_id).await;
//!     eprintln!("{} exited: {:?}", actor_id, exit_reason);
//! }
//! ```
//!
//! ## Terminating an Actor
//!
//! ### "Willful" Termination
//!
//! #### Returning from the Behaviour Function
//!
//! If the actor's behaviour function returns — the actor terminates.
//! The return type of the behaviour function must implement the trait
//! [`Into<Exit>`](crate::actors::Exit).
//!
//! Example:
//! ```
//! use std::convert::Infallible;
//! use agner::actors::{Context, Exit};
//!
//! async fn unit_is_normal_exit(_context: &mut Context<Infallible>, _args:()) {}
//!
//! async fn result_into_exit(_context: &mut Context<Infallible>, _args:()) -> Result<(), Exit> {
//!     Ok(()) // Equivalent to `Err(Exit::normal())`
//! }
//! ```
//!
//! #### Invoking `Context::exit`
//!
//! Example:
//! ```
//! use std::convert::Infallible;
//! use agner::actors::{Context, Exit};
//!
//! async fn normal_exit(context: &mut Context<Infallible>, args: ()) -> Infallible {
//!     context.exit(Exit::normal()).await;
//!     unreachable!()
//! }
//! ```
//!
//! ### Terminating from Outside
//!
//! An actor can be terminated by invoking [`System::exit(&self, ActorID,
//! Exit)`](crate::actors::System::exit).
//!
//! In this case an actor receives an exit-signal. If the actor ["traps
//! exits"](crate::actors::Context::trap_exit), it can perform a graceful shutdown (or even keep
//! running). That is if the exit reason is not [`Exit::kill()`](crate::actors::Exit::kill): in this
//! case the actor just terminates, and its linked actors in their turn receive exit-signals.
//!
//!
//!
//! # Supervision
//!
//! > As this whole project is based on the Erlang/OTP, it should not be a surprise that there
//! > are some (quite a lot of) similarities to the OTP Design Principles, namely:
//! > - [Supervision Trees](https://www.erlang.org/doc/design_principles/des_princ.html#supervision-trees);
//! > - [Special Processes / Starting the Process](https://www.erlang.org/doc/design_principles/spec_proc.html#starting-the-process);
//! > - [Supervisor Behaviour](https://www.erlang.org/doc/design_principles/sup_princ.html).
//!
//! ## Design Principles
//!
//! ### Starting and Stopping
//!
//! TBD:
//! - [`sup::common::start_child`](crate::sup::common::start_child)
//! - [`sup::common::stop_child`](crate::sup::common::stop_child))
//! - [`init_ack`](crate::init_ack)
//!
//! ### Uniform Supervisor
//!
//! TBD:
//! - [uniform supervisor](crate::sup::uniform)
//!
//! ### Mixed Supervisor
//!
//! TBD:
//! - [mixed supervisor](crate::sup::mixed)
//!
//! # Introspection
//!
//! TBD:
//! - [helm](crate::helm)
//!
//! # Testing
//!
//! TBD:
//! - [test-actor](crate::test_actor)

pub use agner_utils as utils;

pub use agner_actors as actors;

#[cfg(feature = "init-ack")]
pub use agner_init_ack as init_ack;

#[cfg(feature = "reg")]
pub use agner_reg as reg;

#[cfg(feature = "sup")]
pub use agner_sup as sup;

#[cfg(feature = "helm")]
pub use agner_helm as helm;

#[cfg(feature = "test-actor")]
pub use agner_test_actor as test_actor;

#[cfg(feature = "log")]
pub use agner_tracing::log;

#[cfg(feature = "log")]
pub use agner_log as log;
