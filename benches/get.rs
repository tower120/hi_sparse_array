use std::collections::BTreeMap;
use criterion::{black_box, Criterion, criterion_group, criterion_main};
use rand::{Rng, SeedableRng};
use rand::seq::SliceRandom;
use hibit_tree::{config, SparseTree, Index};
use hibit_tree::DenseTree;
//use hi_sparse_array::level_block::{Block, ClusterBlock, SmallBlock};
//use hi_sparse_array::Iter;
//use hi_sparse_array::level::{IntrusiveListLevel, SingleBlockLevel};
use hibit_tree::HibitTree;
use hibit_tree::ops::multi_intersection::Data;

const RANGE: usize = 260_000;
const COUNT: usize = /*4000*/100;

/*type Lvl0Block = Block<u64, [u8;64]>;
type Lvl1Block = Block<u64, [u16;64]>;
type Lvl2Block = Block<u64, [u32;64]>;

type CompactLvl1Block = SmallBlock<u64, [u8;1], [u16;64], [u16;7]>;
type CompactLvl2Block = SmallBlock<u64, [u8;1], [u32;64], [u32;7]>;

type ClusterLvl1Block = ClusterBlock<u64, [u16;4], [u16;16]>;*/

#[derive(Default, Clone)]
struct DataBlock(u64);
/*impl Empty for DataBlock{
    fn empty() -> Self {
        Self(0)
    }

    fn is_empty(&self) -> bool {
        todo!()
    }
}*/

type Map = nohash_hasher::IntMap<u32, DataBlock>;
//type Map = ahash::AHashMap<u32, DataBlock>;
type BTree = BTreeMap<u32, DataBlock>;

type CompactArray = DenseTree<DataBlock, 5>;

//type BlockArray = SparseArray<(SingleBlockLevel<Lvl0Block>, IntrusiveListLevel<Lvl1Block>, IntrusiveListLevel<Lvl2Block>), DataBlock>;
type BlockArray = SparseTree<config::width_256::depth_4, DataBlock>;

//type SmallBlockArray = SparseArray<(SingleBlockLevel<Lvl0Block>, IntrusiveListLevel<CompactLvl1Block>, IntrusiveListLevel<CompactLvl2Block>), DataBlock>;
//type SmallBlockArray = SparseArray<config::sbo::width_64::depth_6, DataBlock>;
/*type ClusterBlockArray = SparseArray<(SingleBlockLevel<Lvl0Block>, IntrusiveListLevel<ClusterLvl1Block>), IntrusiveListLevel<DataBlock>>;*/

/*fn cluster_array_get(array: &ClusterBlockArray) -> u64 {
    let mut s = 0;
    for (_, i) in CachingBlockIter::new(array){
        s += i.0;
    }
    s
}*/

fn compact_array_get(array: &CompactArray, indices: &[usize]) -> u64 {
    let mut s = 0;
    for &i in indices{
        //s += unsafe{ array.get_unchecked(i) }.0;
        s += unsafe{ array.get(Index::new_unchecked(i)) }.unwrap_or(&DataBlock(0)).0;
        //s += array.get_or_default(i).0;
    }
    s
}

/*fn small_array_get(array: &SmallBlockArray, indices: &[usize]) -> u64 {
    let mut s = 0;
    for &i in indices{
        s += array.get(i).0;
    }
    s
}*/

fn array_get(array: &BlockArray, indices: &[usize]) -> u64 {
    let mut s = 0;
    for &i in indices{
        unsafe{
        s += array.get(Index::new_unchecked(i))
            //.unwrap_unchecked().0;
            .unwrap_or(&DataBlock(0)).0;
        }
    }
    s
}

fn hashmap_get(array: &Map, indices: &[usize]) -> u64 {
    let mut s = 0;
    for i in indices{
        s += array.get(&(*i as _)).unwrap_or(&DataBlock(0)).0;
    }
    s
}

fn btree_get(array: &BTree, indices: &[usize]) -> u64 {
    let mut s = 0;
    for i in indices{
        s += array.get(&(*i as _)).unwrap_or(&DataBlock(0)).0;
    }
    s
}


pub fn bench_iter(c: &mut Criterion) {
    let mut block_array = BlockArray::default();
    //let mut small_block_array = SmallBlockArray::default();
    let mut compact_array = CompactArray::default();
    /*let mut cluster_block_array = ClusterBlockArray::default();*/
    let mut hashmap = Map::default();
    let mut btree = BTree::default();
    
    let mut rng = rand::rngs::StdRng::seed_from_u64(0xe15bb9db3dee3a0f);
    let mut random_indices = Vec::new();
    
    for _ in 0..COUNT {
        let v = rng.gen_range(0..RANGE);
        random_indices.push(v);
        
        block_array.insert(v, DataBlock(v as _));
        //*small_block_array.get_mut(v) = DataBlock(v as u64);
        *compact_array.get_or_insert(v)= DataBlock(v as u64);
        /* *cluster_block_array.get_or_insert(v) = DataBlock(v as u64);*/
        hashmap.insert(v as _, DataBlock(v as u64));
        btree.insert(v as _, DataBlock(v as u64));
    }
    random_indices.shuffle(&mut rng);

    c.bench_function("level_block array", |b| b.iter(|| array_get(black_box(&block_array), black_box(&random_indices))));
    c.bench_function("compact array", |b| b.iter(|| compact_array_get(black_box(&compact_array), black_box(&random_indices))));
    //c.bench_function("small level_block array", |b| b.iter(|| small_array_get(black_box(&small_block_array), black_box(&random_indices))));
    /*c.bench_function("cluster level_block array", |b| b.iter(|| cluster_array_get(black_box(&cluster_block_array))));*/
    c.bench_function("hashmap", |b| b.iter(|| hashmap_get(black_box(&hashmap), black_box(&random_indices))));
    c.bench_function("btree", |b| b.iter(|| btree_get(black_box(&btree), black_box(&random_indices))));
}

criterion_group!(benches_iter, bench_iter);
criterion_main!(benches_iter);
