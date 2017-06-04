//! Representation and operations on page tables.

use core::ops::{Index, IndexMut};
use core::marker::PhantomData;
use ::arch::x86_64::memory::FrameAllocator;
use super::entry::*;
use super::ENTRY_COUNT;

pub trait TableLevel {}
pub enum Level4 {}
pub enum Level3 {}
pub enum Level2 {}
pub enum Level1 {}
impl TableLevel for Level4 {}
impl TableLevel for Level3 {}
impl TableLevel for Level2 {}
impl TableLevel for Level1 {}
trait HierarchicalLevel: TableLevel {type NextLevel: TableLevel;}
impl HierarchicalLevel for Level4 {type NextLevel = Level3;}
impl HierarchicalLevel for Level3 {type NextLevel = Level2;}
impl HierarchicalLevel for Level2 {type NextLevel = Level1;}

pub const P4: *mut Table<Level4> = 0xffffffff_fffff000 as *mut _;

pub struct Table<L: TableLevel> {
    entries: [Entry; ENTRY_COUNT],
    level: PhantomData<L>,
}

impl<L> Table<L> where L: TableLevel {
    pub fn zero(&mut self) {
        for entry in self.entries.iter_mut() {
            entry.set_unused();
        }
    }
}

impl<L> Table<L> where L: HierarchicalLevel {
    fn next_table_address(&self, index: usize) -> Option<usize> {
        let entry_flags = self[index].flags();
        if entry_flags.contains(PRESENT) && !entry_flags.contains(HUGE_PAGE) {
            let table_address = self as *const _ as usize;
            Some((table_address << 9) | (index << 12))
        } else {
            None
        }
    }

    pub fn next_table(&self, index: usize) -> Option<&Table<L::NextLevel>> {
        self.next_table_address(index)
            .map(|addr| unsafe {&*(addr as *const _)})
    }

    pub fn next_table_mut(&self, index: usize) -> Option<&mut Table<L::NextLevel>> {
        self.next_table_address(index)
            .map(|addr| unsafe {&mut *(addr as *mut _)})
    }

    pub fn next_table_create<A>(&mut self, index: usize, allocator: &mut A) -> &mut Table<L::NextLevel>
            where A: FrameAllocator {
        if self.next_table(index).is_none() {
            assert!(!self.entries[index].flags().contains(HUGE_PAGE),
                "Attempted to create a subtable for a hugepage; we do not currently support hugepages.");
            let frame = allocator.alloc_frame()
                .expect("Attempted to allocate a frame for a subtable, but no frames are available!");

            self.entries[index].set(frame, PRESENT | WRITABLE);
            // Zero the new table
            self.next_table_mut(index).unwrap().zero();
        }

        self.next_table_mut(index).unwrap()
    }
}

impl<L> Index<usize> for Table<L> where L: TableLevel {
    type Output = Entry;

    fn index(&self, index: usize) -> &Entry {
         &self.entries[index]
    }
}

impl<L> IndexMut<usize> for Table<L> where L: TableLevel {
    fn index_mut(&mut self, index: usize) -> &mut Entry {
         &mut self.entries[index]
    }
}
