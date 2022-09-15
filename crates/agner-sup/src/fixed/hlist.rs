// pub mod index_;
pub mod index;
pub mod push;

pub type Nothing = std::convert::Infallible;

pub trait HList {
    const LEN: usize;

    type Head;
    type Tail;
}

#[derive(Debug, Clone, Copy)]
pub struct Nil;

#[derive(Debug, Clone)]
pub struct Cons<H, T>(H, T);

impl HList for Nil {
    const LEN: usize = 0;
    type Head = Nothing;
    type Tail = Nothing;
}

impl<H, T> HList for Cons<H, T>
where
    T: HList,
{
    const LEN: usize = 1 + T::LEN;
    type Head = H;
    type Tail = T;
}
