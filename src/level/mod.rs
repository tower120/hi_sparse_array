mod single_block_level;
mod intrusive_list_level;
mod level;

pub use single_block_level::*;
pub use intrusive_list_level::*;
pub use level::*;

use crate::Empty;

pub trait ILevel: Default {
    // TODO: Now it is always "HiBlock"
    type Block: Empty;
    
    fn blocks(&self) -> &[Self::Block];
    fn blocks_mut(&mut self) -> &mut [Self::Block];
    
    fn insert_empty_block(&mut self) -> usize;
    
    /// # Safety
    ///
    /// block_index and level_block emptiness are not checked.
    unsafe fn remove_empty_block_unchecked(&mut self, block_index: usize);
}

