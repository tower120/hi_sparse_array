use std::marker::PhantomData;
use std::borrow::Borrow;
use std::mem::MaybeUninit;
use std::ptr::NonNull;
use std::slice;
use arrayvec::ArrayVec;
use crate::{BitBlock, LazySparseHierarchy, MonoSparseHierarchy, MultiSparseHierarchy, MultiSparseHierarchyTypes, SparseHierarchyData, SparseHierarchyStateTypes, SparseHierarchyTypes};
use crate::const_utils::{ConstArray, ConstArrayType, ConstInteger};
use crate::sparse_hierarchy::{SparseHierarchy, SparseHierarchyState};
use crate::utils::{Array, Borrowable, Ref, Take};

/// Intersection between all iterator items.
///
/// All returned iterators are [ExactSizeIterator]. 
pub struct MultiIntersection<Iter> {
    iter: Iter,
}

type IterItem<Iter> = <<Iter as Iterator>::Item as Ref>::Type;
type IterItemState<'item, Iter> = <IterItem<Iter> as SparseHierarchyTypes<'item>>::State;

impl<'item, 'this, Iter, T> SparseHierarchyTypes<'this> for MultiIntersection<Iter>
where
    Iter: Iterator<Item = &'item T> + Clone,
    T: SparseHierarchy + 'item
{
    type Data  = ResolveIter<'item, Iter>;
    type DataUnchecked = ResolveIterUnchecked<Iter>;
    type State = MultiIntersectionState<'this, 'item, Iter>;
}

impl<'i, Iter, T> SparseHierarchy for MultiIntersection<Iter>
where
    Iter: Iterator<Item = &'i T> + Clone,
    T: SparseHierarchy + 'i
{
    const EXACT_HIERARCHY: bool = false;
    
    type LevelCount = T::LevelCount;
    type LevelMask  = T::LevelMask;

    #[inline]
    unsafe fn data(&self, index: usize, level_indices: &[usize]) 
        -> Option<<Self as SparseHierarchyTypes<'_>>::Data> 
    {
        // There are few ways to do it:
        // 1. Iterate, get data() and build resolve value on the fly. As
        //    soon as None meet - throw away half-built resolve value and 
        //    return None. 
        //    This should be the fastest one, when intersection is
        //    successful most of the time. Thou it may be not the best one
        //    from user perspective, since resolve function will act "special"
        //    for get() operations.
        // 2. Iterate, get data() and STORE it. If we do not meet None - pass 
        //    stored data to resolve function.
        // 3. Contains + get_unchecked. We traverse hierarchy TWICE.
        
        // Variant 1 implementation.
/*        {
            if self.iter.clone().next().is_none(){
                return None;
            }
            
            let mut not_intersects = false;
            let resolve = (self.f)(
                MultiIntersectionResolveIter::stateless(
                    ResolveIter {
                        index, 
                        level_indices, 
                        iter: self.iter.clone(), 
                        not_intersects: &mut not_intersects
                    }
                )
            );
            if not_intersects{
                None
            } else {
                Some(resolve)
            }
        }*/
        
        // Variant 2 implementation.
        //
        // Slower 20% than variant 1 on plain data. Could be more on something
        // more complex. (But we expect that generative SparseHierarchies will not
        // be used for heavy objects)
        //
        // Performs poorly with Vec storage instead of ArrayVec (thou if it 
        // does not fit stack - overhead will probably be neglectable).
        //
        // But no "special cases" from user perspective.
        {
            let mut datas: ArrayVec<_, N> = Default::default();
            for array in self.iter.clone(){
                // TODO: This is only OK, if:
                //
                //     SparseHierarchy<Data:'static>
                //     ||
                //     Iterator<Item = &impl SparseHierarchy>
                //
                //  Or just accept only Iterator<Item = &impl SparseHierarchy> instead of Borrowable
                
                // TODO: AS-IS this is wrong, if self.iter returns arrays as values,
                //       while array.data() contains pointer/reference to array.
                //
                let array = NonNull::from(array.borrow()); // drop borrow lifetime
                let data = unsafe{ array.as_ref().data(index, level_indices) };
                if let Some(data) = data{
                    datas.push(data);
                } else {
                    return None;
                }
            }
            let resolve = ResolveIter {
                items: datas.into_iter()
            };
            Some(resolve)
        }

        // Variant 3 implementation.
        // Performance degrades linearly with depth increase.
        /*{
            for array in self.iter.clone(){
                let array = NonNull::from(array.borrow()); // drop borrow lifetime
                let data = unsafe{ array.as_ref().data(index, level_indices) };
                if data.is_none(){
                    return None;
                }
            }
            
            let resolve = (self.f)(
                MultiIntersectionResolveIter::stateless(
                    ResolveIter {
                        index,
                        level_indices,
                        iter: self.iter.clone(),
                    }
                )
            );
            Some(resolve)            
        }*/
    }

    #[inline]
    unsafe fn data_unchecked<'a>(&'a self, index: usize, level_indices: &'a [usize]) 
        -> <Self as SparseHierarchyTypes<'a>>::DataUnchecked
    {
        ResolveIterUnchecked {
            index, 
            level_indices: Array::from_fn(|i| unsafe{ *level_indices.get_unchecked(i) }), 
            iter: self.iter.clone(),
        }
    }
}

use data_resolve_v2::ResolveIter;

/*mod data_resolve_v1 {
    use super::*;
    
    pub struct ResolveIter<'a, Iter>
    where
        Iter: Iterator<Item: Borrowable<Borrowed: SparseHierarchy>> + 'a,
    {
        pub index: usize, 
        pub level_indices: &'a [usize],
        pub iter: Iter,
        pub not_intersects: &'a mut bool
    }
    impl<'a, Iter> Iterator for ResolveIter<'a, Iter>
    where
        Iter: Iterator<Item: Borrowable<Borrowed: SparseHierarchy>> + 'a,
    {
        type Item = <IterItem<Iter> as SparseHierarchy>::Data<'a>;
    
        #[inline]
        fn next(&mut self) -> Option<Self::Item> {
            if let Some(array) = self.iter.next(){
                let array = NonNull::from(array.borrow()); // drop borrow lifetime
                if let Some(data) = unsafe{ array.as_ref().data(self.index, self.level_indices) } {
                    return Some(data);
                }
                *self.not_intersects = true;
            }
            None
        }
    }
    impl<'a, Iter> Drop for ResolveIter<'a, Iter>
    where
        Iter: Iterator<Item: Borrowable<Borrowed: SparseHierarchy>> + 'a,
    {
        #[inline]
        fn drop(&mut self) {
            if *self.not_intersects{
                return;
            }
            // search if there are any non-intersected elements left.
            self.fold((), |_, _|());
        }
    }
}
*/
mod data_resolve_v2 {
    use super::*;
    
    // Theoretically we could "somehow" extract 'item lifetime from Iter.
    // But for the sake of sanity - we just pass it.
    pub struct ResolveIter<'item, Iter>
    where
        Iter: Iterator<Item: Ref<Type: SparseHierarchy>>,
    {
        pub items: arrayvec::IntoIter<<IterItem<Iter> as SparseHierarchyTypes<'item>>::Data, N>
    }
    impl<'item, Iter> Iterator for ResolveIter<'item, Iter>
    where
        Iter: Iterator<Item: Ref<Type: SparseHierarchy>>,
    {
        type Item = <IterItem<Iter> as SparseHierarchyTypes<'item>>::Data;
    
        #[inline]
        fn next(&mut self) -> Option<Self::Item> {
            self.items.next()
        }

        #[inline]
        fn size_hint(&self) -> (usize, Option<usize>) {
            self.items.size_hint()
        }
    }
    
    impl<'item, Iter> ExactSizeIterator for ResolveIter<'item, Iter>
    where
        Iter: Iterator<Item: Ref<Type: SparseHierarchy>>
    {}
}

/*mod data_resolve_v3 {
    use super::*;
    
    pub struct ResolveIter<'a, Iter>
    where
        Iter: Iterator<Item: Borrowable<Borrowed: SparseHierarchy>> + 'a,
    {
        pub index: usize, 
        pub level_indices: &'a [usize],
        pub iter: Iter,
    }
    impl<'a, Iter> Iterator for ResolveIter<'a, Iter>
    where
        Iter: Iterator<Item: Borrowable<Borrowed: SparseHierarchy>> + 'a,
    {
        type Item = <IterItem<Iter> as SparseHierarchy>::Data<'a>;
    
        #[inline]
        fn next(&mut self) -> Option<Self::Item> {
            if let Some(array) = self.iter.next(){
                let array = NonNull::from(array.borrow()); // drop borrow lifetime
                Some(unsafe{ array.as_ref().data_unchecked(self.index, self.level_indices) })
            } else {
                None
            }
        }
    }
}
*/

pub struct ResolveIterUnchecked<Iter> 
where
    Iter: Iterator<Item: Ref<Type: SparseHierarchy>>,
{
    index: usize, 
    // This is copy from level_indices &[usize]. 
    // Compiler optimize away the very act of cloning and directly use &[usize].
    // At least, if value used immediately, and not stored for latter use. 
    level_indices: ConstArrayType<usize, <IterItem<Iter> as SparseHierarchy>::LevelCount>,
    iter: Iter,
}
impl<'item, Iter, T> Iterator for ResolveIterUnchecked<Iter>
where
    Iter: Iterator<Item = &'item T> + Clone,
    T: SparseHierarchy + 'item,
{
    type Item = <IterItem<Iter> as SparseHierarchyTypes<'item>>::DataUnchecked;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.iter
            .next()
            .map(|array| unsafe {
                array.data_unchecked(self.index, self.level_indices.as_ref())
            })
    }

    #[inline]
    fn fold<B, F>(self, init: B, mut f: F) -> B
    where
        Self: Sized,
        F: FnMut(B, Self::Item) -> B,
    {
        self.iter.fold(init, |init, array| unsafe {
            let data = array.data_unchecked(self.index, self.level_indices.as_ref());
            f(init, data)
        })
    }
    
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl<'item, Iter, T> ExactSizeIterator for ResolveIterUnchecked<Iter>
where
    Iter: Iterator<Item = &'item T> + Clone,
    T: SparseHierarchy + 'item,
{}

const N: usize = 32;
type StatesItem<'item, Iter> = IterItemState<'item, Iter>; 

// TODO: rename to State
pub struct MultiIntersectionState<'src, 'item, I>
where
    I: Iterator<Item: Ref<Type: SparseHierarchy>>
{
    states: ArrayVec<StatesItem<'item, I>, N>,    
    empty_below_n: usize,
    terminal_node_mask: <IterItem<I> as SparseHierarchy>::LevelMask,
    phantom_data: PhantomData<(&'src MultiIntersection<I>)>
}


impl<'this, 'src, 'item, Iter> SparseHierarchyStateTypes<'this> for MultiIntersectionState<'src, 'item, Iter>
where
    Iter: Iterator<Item: Ref<Type: SparseHierarchy>>
{
    type Data = StateResolveIter<'this, 'item, Iter>;
}

impl<'src, 'item, Iter, T> SparseHierarchyState<'src> for MultiIntersectionState<'src, 'item, Iter>
where
    Iter: Iterator<Item = &'item T> + Clone,
    T: SparseHierarchy + 'item
{
    type Src = MultiIntersection<Iter>;

    #[inline]
    fn new(src: &'src Self::Src) -> Self {
        let states = ArrayVec::from_iter(
            src.iter.clone()
                .map(|array|{
                    SparseHierarchyState::new(array)
                })
        );
        
        Self {
            states,
            empty_below_n: usize::MAX,
            terminal_node_mask: BitBlock::zero(),
            phantom_data: PhantomData,
        }        
    }

    #[inline]
    unsafe fn select_level_node<N: ConstInteger>(
        &mut self, src: &'src Self::Src, level_n: N, level_index: usize
    ) -> <Self::Src as SparseHierarchy>::LevelMask {
        // if we know that upper levels returned empty - return early.
        if N > self.empty_below_n {
            return BitBlock::zero(); 
        }
        
        let mut states_iter = self.states.iter_mut();
        let mut array_iter  = src.iter.clone();
        
        let mut acc_mask = 
            if let Some(array_state) = states_iter.next(){
                let array = array_iter.next().unwrap_unchecked();
                array_state.select_level_node(array, level_n, level_index)
            } else {
                return BitBlock::zero();
            };
        
        for array_state in states_iter {
            let array = array_iter.next().unwrap_unchecked();
            let mask = array_state.select_level_node(
                array, level_n, level_index
            );
            acc_mask &= mask;
        }
        
        self.empty_below_n = if acc_mask.is_zero(){
             N
        } else {
            usize::MAX
        };
        
        /*const*/ if N::VALUE == <Self::Src as SparseHierarchy>::LevelCount::VALUE - 1 {
            self.terminal_node_mask = acc_mask.clone(); 
        }
        
        acc_mask
    }

    #[inline]
    unsafe fn select_level_node_unchecked<N: ConstInteger> (
        &mut self, src: &'src Self::Src, level_n: N, level_index: usize
    ) -> <Self::Src as SparseHierarchy>::LevelMask {
        // TODO: Almost the same as in checked version. Reuse somehow. 
        let mut states_iter = self.states.iter_mut();
        let mut array_iter  = src.iter.clone();
        
        let mut acc_mask = 
            if let Some(array_state) = states_iter.next() {
                let array = array_iter.next().unwrap_unchecked();
                array_state.select_level_node_unchecked(array, level_n, level_index)
            } else {
                return BitBlock::zero();
            };
        
        for array_state in states_iter {
            let array = array_iter.next().unwrap_unchecked();
            let mask = array_state.select_level_node_unchecked(
                array, level_n, level_index
            );
            acc_mask &= mask;
        }            
        
        acc_mask
    }

    #[inline]
    unsafe fn data<'a>(&'a self, this: &'src Self::Src, level_index: usize) 
        -> Option<<Self as SparseHierarchyStateTypes<'a>>::Data> 
    {
        if !self.terminal_node_mask.get_bit(level_index){
            return None;
        }
        
        Some(self.data_unchecked(this, level_index))
    }

    #[inline]
    unsafe fn data_unchecked<'a>(
        &'a self, src: &'src Self::Src, level_index: usize
    ) -> <Self as SparseHierarchyStateTypes<'a>>::Data {
        StateResolveIter { 
            level_index,
            array_iter: src.iter.clone(),
            states_iter: self.states.iter(),
        }
    }
}

// TODO: rename
// States slice to Data iterator adapter.
pub struct StateResolveIter<'state, 'item, I>
where
    I: Iterator<Item: Ref<Type: SparseHierarchy>>
{
    level_index: usize,
    array_iter: I,
    states_iter: slice::Iter<'state, StatesItem<'item, I>>,
}

/// Iterator for [MultiIntersection] resolve function.
/// 
/// Prefer using [fold]-based[^1] operations over [next]-ing.
///
/// [^1]: Such as [for_each], [sum], etc... 
impl<'state, 'item, I, T> Iterator for StateResolveIter<'state, 'item, I>
where
    I: Iterator<Item = &'item T> + Clone,
    T: SparseHierarchy + 'item
{
    type Item = <IterItemState<'item, I> as SparseHierarchyStateTypes<'state>>::Data;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        // Compiler optimizes away additional branching here.
        self.states_iter
            .next()
            .map(|array_state| unsafe { 
                let array = self.array_iter.next().unwrap_unchecked();
                array_state.data_unchecked(array, self.level_index)
            })
    }

    #[inline]
    fn fold<B, F>(mut self, mut init: B, mut f: F) -> B
    where
        Self: Sized,
        F: FnMut(B, Self::Item) -> B,
    {
        let level_index = self.level_index;
        for array_state in self.states_iter {
            let data = unsafe{
                let array = self.array_iter.next().unwrap_unchecked();
                array_state.data_unchecked(array, level_index) 
            };
            init = f(init, data);
        }
        init
    }
    
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.states_iter.size_hint()
    }
}

impl<'state, 'item, I, T> ExactSizeIterator for StateResolveIter<'state, 'item, I>
where
    I: Iterator<Item = &'item T> + Clone,
    T: SparseHierarchy + 'item
{}

impl<'item, 'this, Iter, T> MultiSparseHierarchyTypes<'this> for MultiIntersection<Iter>
where
    Iter: Iterator<Item = &'item T> + Clone,
    T: MonoSparseHierarchy + 'item
{ 
    type IterItem = SparseHierarchyData<'item, T>; 
}

impl<'item, Iter, T> MultiSparseHierarchy for MultiIntersection<Iter>
where
    Iter: Iterator<Item = &'item T> + Clone,
    T: MonoSparseHierarchy + 'item
{} 

impl<Iter> LazySparseHierarchy for MultiIntersection<Iter>
where
    MultiIntersection<Iter>: SparseHierarchy
{}

impl<Iter> Borrowable for MultiIntersection<Iter>{ type Borrowed = Self; }

#[inline]
pub fn multi_intersection<Iter>(iter: Iter) 
    -> MultiIntersection<Iter>
where
    Iter: Iterator<Item: Ref<Type:SparseHierarchy>> + Clone,
{
    MultiIntersection{ iter }
}

#[cfg(test)]
mod tests{
    use itertools::assert_equal;
    use crate::compact_sparse_array::CompactSparseArray;
    use crate::sparse_hierarchy::SparseHierarchy;
    use crate::utils::LendingIterator;
    use super::multi_intersection;

    #[test]
    fn smoke_test(){
        type Array = CompactSparseArray<usize, 3>;
        let mut a1 = Array::default();
        let mut a2 = Array::default();
        let mut a3 = Array::default();
        
        *a1.get_or_insert(10) = 10;
        *a1.get_or_insert(15) = 15;
        *a1.get_or_insert(200) = 200;
        
        *a2.get_or_insert(100) = 100;
        *a2.get_or_insert(15)  = 15;
        *a2.get_or_insert(200) = 200;
        
        *a3.get_or_insert(300) = 300;
        *a3.get_or_insert(15)  = 15;
        
        let arrays = [a1, a2, a3];
        
        let intersection = multi_intersection(arrays.iter());
        
        let mut iter = intersection.iter();
        while let Some((index, values)) = iter.next(){
            let values: Vec<_> = values.collect();
            println!("{:?}", values);
        }
        
        assert_equal( 
            intersection.get(15).unwrap(),
            vec![arrays[0].get(15).unwrap(), arrays[1].get(15).unwrap(), arrays[2].get(15).unwrap()]
        );
        assert!( intersection.get(200).is_none() );
        assert_equal(unsafe{ intersection.get_unchecked(15) }, intersection.get(15).unwrap());
    }

}
