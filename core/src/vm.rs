use core::mem::MaybeUninit;
use fixed_slice_vec::FixedSliceVec;

#[derive(Debug, PartialEq)]
pub enum Token {
    Any,
}

#[derive(Clone, Copy, Debug)]
pub enum MemoryCell {}

pub struct Machine<'a> {
    memory: FixedSliceVec<'a, MemoryCell>,
    //stacks: FixedSliceVec<'a, FixedSliceVec<'a, MemoryCell>>,
}

impl<'a> Machine<'a> {
    pub fn new(
        memory: &'a mut [MaybeUninit<MemoryCell>],
        //stacks: &'a mut [MaybeUninit<FixedSliceVec<'a, MemoryCell>>],
    ) -> Self {
        Self {
            memory: FixedSliceVec::new(memory),
            //stacks: FixedSliceVec::new(stacks),
        }
    }
}
