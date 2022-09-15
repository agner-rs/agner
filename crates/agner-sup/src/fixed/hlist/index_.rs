use crate::fixed::hlist::Cons;

pub trait Index<I> {
    type Item;
    fn get(&self) -> &Self::Item;
}

pub trait IndexMut<I>: Index<I> {
    fn get_mut(&mut self) -> &mut Self::Item;
}

pub trait Idx {
    const IDX: usize;
}

#[derive(Debug, Clone, Copy)]
pub struct Here;

#[derive(Debug, Clone, Copy)]
pub struct There<I>(I);

impl Idx for Here {
    const IDX: usize = 0;
}
impl<I> Idx for There<I>
where
    I: Idx,
{
    const IDX: usize = 1 + I::IDX;
}

impl<H, T> Index<Here> for Cons<H, T> {
    type Item = H;
    fn get(&self) -> &Self::Item {
        &self.0
    }
}
impl<H, T, I> Index<There<I>> for Cons<H, T>
where
    T: Index<I>,
{
    type Item = <T as Index<I>>::Item;
    fn get(&self) -> &Self::Item {
        self.1.get()
    }
}
impl<H, T> IndexMut<Here> for Cons<H, T> {
    fn get_mut(&mut self) -> &mut Self::Item {
        &mut self.0
    }
}
impl<H, T, I> IndexMut<There<I>> for Cons<H, T>
where
    T: IndexMut<I>,
{
    fn get_mut(&mut self) -> &mut Self::Item {
        self.1.get_mut()
    }
}


