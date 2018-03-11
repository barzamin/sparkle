//! Controls Sparkle's IDT.

use x86::structures::idt::{Idt, ExceptionStackFrame};
use x86::structures::tss::TaskStateSegment;
use x86::structures::gdt::SegmentSelector;
use x86::VirtualAddress;
use spin::Once;

use arch::x86_64::memory::MemoryController;

mod gdt;

use self::gdt::Gdt;


const IST_DOUBLE_FAULT: usize = 0;

lazy_static! {
    static ref IDT: Idt = {
        let mut idt = Idt::new();

        idt.breakpoint.set_handler_fn(breakpoint_handler);
        unsafe {
            idt.double_fault.set_handler_fn(double_fault_handler)
                .set_stack_index(IST_DOUBLE_FAULT as u16);
        }

        idt
    };
}

static TSS: Once<TaskStateSegment> = Once::new();
static GDT: Once<Gdt> = Once::new();

pub fn init(memory_controller: &mut MemoryController) {
    use x86::instructions::segmentation::set_cs;
    use x86::instructions::tables::load_tss;

    let double_fault_stack = memory_controller.alloc_stack(1)
        .expect("could not allocate stack for double faulting");

    let tss = TSS.call_once(|| {
        let mut tss = TaskStateSegment::new();
        tss.interrupt_stack_table[IST_DOUBLE_FAULT] =
            VirtualAddress(double_fault_stack.top());

        tss
    });

    let mut code_selector = SegmentSelector(0);
    let mut tss_selector = SegmentSelector(0);
    let gdt = GDT.call_once(|| {
        let mut gdt = Gdt::new();

        code_selector = gdt.add_entry(gdt::Descriptor::kernel_code_segment());
        tss_selector = gdt.add_entry(gdt::Descriptor::tss_segment(&tss));

        gdt
    });
    gdt.load();

    unsafe {
        set_cs(code_selector);
        load_tss(tss_selector);
    }

    IDT.load();
}

extern "x86-interrupt" fn breakpoint_handler(stack_frame: &mut ExceptionStackFrame) {
    println!("int[3]: trap breakpoint:\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn double_fault_handler(
    stack_frame: &mut ExceptionStackFrame, _error_code: u64)
{
    println!("int[8]: fault: double:\n{:#?}", stack_frame);

    loop {}
}