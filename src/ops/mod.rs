pub(crate) mod map;
pub use map::{Map, MapFunction};


pub(crate) mod multi_map_fold;
pub use multi_map_fold::MultiMapFold;


pub(crate) mod intersection;
pub use intersection::Intersection;


pub(crate) mod union;
pub use union::Union;


pub(crate) mod _multi_intersection;
pub mod multi_intersection {
    pub use super::_multi_intersection::{
        Data,
        DataUnchecked,
        Cursor,
        CursorData
    };
}
pub use _multi_intersection::MultiIntersection;


pub(crate) mod _multi_union;
pub mod multi_union {
    pub use super::_multi_union::{
        Data,
        DataUnchecked,
        Cursor,
        CursorData,
    };
}
pub use _multi_union::MultiUnion;