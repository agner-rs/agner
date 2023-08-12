mod reg;
pub use reg::{new, RegGuard, RegRx, RegTx};

#[cfg(test)]
mod tests;
