pub trait ArgsFactory: Send + Sync + 'static {
    type Input: Send + Sync + 'static;
    type Output: Send + Sync + 'static;

    fn make_args(&mut self, args: Self::Input) -> Self::Output;
}
