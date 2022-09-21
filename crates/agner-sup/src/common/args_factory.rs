pub trait ArgsFactory: Send + Sync + 'static {
    type Output: Send + Sync + 'static;

    fn make_args(&mut self) -> Self::Output;
}
