// #![feature(type_alias_impl_trait)]
// #![feature(generic_associated_types)]
#![cfg(test)]

const ITERATIONS: usize = 1_000_000;
const PAYLOAD_SIZE: usize = 64;

mod b01_1_futures_mpsc;
mod b01_2_tokio_mpsc;
mod b02_1_static_dispatch;
mod b02_2_dynamic_dispatch;
mod b03_1_async_trait;
// mod b03_2_type_alias_impl_trait;
// mod b03_3_boxed;
