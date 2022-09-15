use super::{Cons, HList, Nil};

pub trait PushFront<H>: HList + Sized {
    fn push_front(self, head: H) -> Cons<H, Self> {
        Cons(head, self)
    }
}

pub trait PushBack<I>: HList {
    type Out;
    fn push_back(self, item: I) -> Self::Out;
}

impl<H, L> PushFront<H> for L where L: HList {}

impl<I> PushBack<I> for Nil {
    type Out = Cons<I, Self>;

    fn push_back(self, item: I) -> Self::Out {
        Cons(item, self)
    }
}

impl<I, H, T> PushBack<I> for Cons<H, T>
where
    T: PushBack<I>,
{
    type Out = Cons<H, <T as PushBack<I>>::Out>;

    fn push_back(self, item: I) -> Self::Out {
        Cons(self.0, self.1.push_back(item))
    }
}
