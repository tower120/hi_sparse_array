use std::marker::PhantomData;
use std::borrow::Borrow;
use std::ops::{BitAnd, BitOr};
use crate::const_utils::{ConstArray, ConstArrayType, ConstInteger};
use crate::hibit_tree::{HibitTree, HibitTreeCursor};
use crate::{BitBlock, LazyHibitTree, HibitTreeCursorTypes, HibitTreeTypes};
use crate::bit_queue::BitQueue;
use crate::utils::{Array, Borrowable};

pub struct Union<S0, S1>{
    s0: S0,
    s1: S1,
}

impl<'this, S0, S1> HibitTreeTypes<'this> for Union<S0, S1>
where
    S0: Borrowable<Borrowed: HibitTree>,
    S1: Borrowable<Borrowed: HibitTree<
        LevelCount = <S0::Borrowed as HibitTree>::LevelCount,
        LevelMask  = <S0::Borrowed as HibitTree>::LevelMask,
    >>,
{
    type Data = (
        Option<<S0::Borrowed as HibitTreeTypes<'this>>::Data>,
        Option<<S1::Borrowed as HibitTreeTypes<'this>>::Data>
    );
    
    type DataUnchecked = Self::Data;
    
    type Cursor = Cursor<'this, S0, S1>;
}

impl<S0, S1> HibitTree for Union<S0, S1>
where
    S0: Borrowable<Borrowed: HibitTree>,
    S1: Borrowable<Borrowed: HibitTree<
        LevelCount = <S0::Borrowed as HibitTree>::LevelCount,
        LevelMask  = <S0::Borrowed as HibitTree>::LevelMask,
    >>,
{
    /// true if S0 & S1 are EXACT_HIERARCHY.
    const EXACT_HIERARCHY: bool = <S0::Borrowed as HibitTree>::EXACT_HIERARCHY 
                                & <S1::Borrowed as HibitTree>::EXACT_HIERARCHY;
    
    type LevelCount = <S0::Borrowed as HibitTree>::LevelCount;
    type LevelMask  = <S0::Borrowed as HibitTree>::LevelMask;

    #[inline]
    unsafe fn data(&self, index: usize, level_indices: &[usize]) 
        -> Option<<Self as HibitTreeTypes<'_>>::Data> 
    {
        let d0 = self.s0.borrow().data(index, level_indices);
        let d1 = self.s1.borrow().data(index, level_indices);
        if d0.is_none() & d1.is_none(){
            None
        } else {
            Some((d0, d1))
        }
    }

    #[inline]
    unsafe fn data_unchecked(&self, index: usize, level_indices: &[usize]) 
        -> <Self as HibitTreeTypes<'_>>::Data
    {
        self.data(index, level_indices).unwrap_unchecked()
    }
}

/// [S::Mask; S::DEPTH]
type Masks<S> = ConstArrayType<
    <<S as Borrowable>::Borrowed as HibitTree>::LevelMask,
    <<S as Borrowable>::Borrowed as HibitTree>::LevelCount,
>;

pub struct Cursor<'src, S0, S1>
where
    S0: Borrowable<Borrowed: HibitTree>,
    S1: Borrowable<Borrowed: HibitTree>,
{
    s0: <S0::Borrowed as HibitTreeTypes<'src>>::Cursor, 
    s1: <S1::Borrowed as HibitTreeTypes<'src>>::Cursor,
    phantom: PhantomData<&'src Union<S0, S1>>
}

impl<'this, 'src, S0, S1> HibitTreeCursorTypes<'this> for Cursor<'src, S0, S1>
where
    S0: Borrowable<Borrowed: HibitTree>,
    S1: Borrowable<Borrowed: HibitTree>,
{
    type Data = (
        Option<<<S0::Borrowed as HibitTreeTypes<'src>>::Cursor as HibitTreeCursorTypes<'this>>::Data>,
        Option<<<S1::Borrowed as HibitTreeTypes<'src>>::Cursor as HibitTreeCursorTypes<'this>>::Data>
    );
}

impl<'src, S0, S1> HibitTreeCursor<'src> for Cursor<'src, S0, S1>
where
    S0: Borrowable<Borrowed: HibitTree>,
    S1: Borrowable<Borrowed: HibitTree<
        LevelCount = <S0::Borrowed as HibitTree>::LevelCount,
        LevelMask  = <S0::Borrowed as HibitTree>::LevelMask,
    >>
{
    type Src = Union<S0, S1>;

    #[inline]
    fn new(src: &'src Self::Src) -> Self {
        Self{
            s0: HibitTreeCursor::new(src.s0.borrow()), 
            s1: HibitTreeCursor::new(src.s1.borrow()),
            phantom: PhantomData
        }
    }

    #[inline]
    unsafe fn select_level_node<N: ConstInteger>(
        &mut self, this: &'src Self::Src, level_n: N, level_index: usize
    ) -> <Self::Src as HibitTree>::LevelMask {
        // unchecked version already deal with non-existent elements
        self.select_level_node_unchecked(this, level_n, level_index)
    }

    #[inline]
    unsafe fn select_level_node_unchecked<N: ConstInteger> (
        &mut self, this: &'src Self::Src, level_n: N, level_index: usize
    ) -> <Self::Src as HibitTree>::LevelMask {
        let mask0 = self.s0.select_level_node(
            this.s0.borrow(), level_n, level_index,
        );

        let mask1 = self.s1.select_level_node(
            this.s1.borrow(), level_n, level_index,
        );

        // mask0.take_or_clone() |= mask1.borrow() 
        {
            let mut mask = mask0;
            mask |= mask1.borrow();
            mask
        }
    }

    #[inline]
    unsafe fn data<'a>(&'a self, this: &'src Self::Src, level_index: usize) 
        -> Option<<Self as HibitTreeCursorTypes<'a>>::Data> 
    {
        let d0 = self.s0.data(this.s0.borrow(), level_index);
        let d1 = self.s1.data(this.s1.borrow(), level_index);
        if d0.is_none() & d1.is_none(){
            None
        } else {
            Some((d0, d1))
        }
    }

    #[inline]
    unsafe fn data_unchecked<'a>(&'a self, this: &'src Self::Src, level_index: usize) 
        -> <Self as HibitTreeCursorTypes<'a>>::Data 
    {
        self.data(this, level_index).unwrap_unchecked()
    }
}

impl<S0, S1> LazyHibitTree for Union<S0, S1>
where
    Union<S0, S1>: HibitTree
{}

impl<S0, S1> Borrowable for Union<S0, S1>{ type Borrowed = Self; }

#[inline]
pub fn union<S0, S1>(s0: S0, s1: S1) -> Union<S0, S1>
where
    // bounds needed here for F's arguments auto-deduction
    S0: Borrowable<Borrowed: HibitTree>,
    S1: Borrowable<Borrowed: HibitTree<
        LevelCount = <S0::Borrowed as HibitTree>::LevelCount,
        LevelMask  = <S0::Borrowed as HibitTree>::LevelMask,
    >>
{
    Union { s0, s1 }
}

#[cfg(test)]
mod tests{
    use itertools::assert_equal;
    use crate::dense_tree::DenseTree;
    use crate::map;
    use crate::ops::union::union;
    use crate::hibit_tree::HibitTree;

    #[test]
    fn smoke_test(){
        type Array = DenseTree<usize, 3>;
        let mut a1 = Array::default();
        let mut a2 = Array::default();
        
        *a1.get_or_insert(10) = 10;
        *a1.get_or_insert(15) = 15;
        *a1.get_or_insert(200) = 200;
        
        *a2.get_or_insert(100) = 100;
        *a2.get_or_insert(15)  = 15;
        *a2.get_or_insert(200) = 200;        

        // test with map
        let union = map(union(&a1, &a2), |(i0, i1): (Option<&usize>, Option<&usize>)| {
            i0.unwrap_or(&0) + i1.unwrap_or(&0)
        });
        
        assert_eq!(unsafe{ union.get_unchecked(200) }, 400);
        assert_eq!(union.get(15), Some(30));
        assert_eq!(union.get(10), Some(10));
        assert_eq!(union.get(20), None);
        
        assert_equal(union.iter(), [(10, 10), (15, 30), (100, 100), (200, 400)]);
    }
}