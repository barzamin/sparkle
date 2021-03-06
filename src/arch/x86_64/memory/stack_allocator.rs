use super::paging::table::EntryFlags;
use super::paging::{ActivePageTable, Frame, FrameAllocator, Page, PageIter};

/// A stack.
///
/// # Notes
/// x86 stacks start at a `top` address (higher in memory than the `bottom` address) and grow downwards to `bottom`
#[derive(Debug)]
pub struct Stack {
    top: usize,
    bottom: usize,
}

impl Stack {
    fn new(top: usize, bottom: usize) -> Stack {
        assert!(
            top > bottom,
            "Stack top must be higher in memory than the bottom"
        );

        Stack { top, bottom }
    }

    /// Returns the address of the top of this stack.
    pub fn top(&self) -> usize {
        self.top
    }

    /// Returns the address of the bottom of this stack.
    pub fn bottom(&self) -> usize {
        self.bottom
    }
}

/// An allocator which allocates [Stack]s.
pub struct StackAllocator {
    range: PageIter,
}

impl StackAllocator {
    /// Creates a new stack allocator, allocating stacks in the given `page_range`.
    pub fn new(page_range: PageIter) -> StackAllocator {
        StackAllocator { range: page_range }
    }

    /// Allocate a stack.
    ///
    /// Note: `size` is given in pages.
    pub fn alloc_stack<A>(
        &mut self,
        active_table: &mut ActivePageTable,
        frame_alloc: &mut A,
        size: usize,
    ) -> Option<Stack>
    where
        A: FrameAllocator,
    {
        // zero-size stacks are nonsensical
        if size == 0 {
            return None;
        }

        let mut range = self.range.clone();

        // try to alloc stack, guard pages
        let guard_page = range.next();
        let stack_start = range.next();
        let stack_end = if size == 1 {
            stack_start
        } else {
            range.nth(size - 2)
        };

        match (guard_page, stack_start, stack_end) {
            (Some(_), Some(start), Some(end)) => {
                // writeback
                self.range = range;

                // map stack pages -> physical frames
                for page in Page::range_inclusive(start, end) {
                    active_table.map(page, EntryFlags::WRITABLE, frame_alloc);
                }

                // create a new stack
                let top = end.start_address() + Frame::SIZE;

                Some(Stack::new(top, start.start_address()))
            }
            _ => None, // whoops not enough frames
        }
    }
}
