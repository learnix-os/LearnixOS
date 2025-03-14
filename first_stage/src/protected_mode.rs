use core::{arch::asm, mem::size_of};

use crate::constants::SECOND_STAGE_OFFSET;

static GDT: GdtProtectedMode = GdtProtectedMode::new();

#[repr(C)]
pub struct GdtProtectedMode {
    zero: u64,
    code: u64,
    data: u64,
}

impl GdtProtectedMode {
    const fn new() -> Self {
        let limit = {
            let limit_low = 0xffff;
            let limit_high = 0xf << 48;
            limit_high | limit_low
        };
        let access_common = {
            let present = 1 << 47;
            let user_segment = 1 << 44;
            let read_write = 1 << 41;
            present | user_segment | read_write
        };
        let protected_mode = 1 << 54;
        let granularity = 1 << 55;
        let base_flags = protected_mode | granularity | access_common | limit;
        let executable = 1 << 43;
        Self {
            zero: 0,
            code: base_flags | executable,
            data: base_flags,
        }
    }

    fn clear_interrupts_and_load(&'static self) {
        let pointer = GdtPointer {
            base: self,
            limit: (3 * size_of::<u64>() - 1) as u16,
        };

        unsafe {
            asm!("cli", "lgdt [{}]", in(reg) &pointer, options(readonly, nostack, preserves_flags));
        }
    }
}

#[repr(C, packed(2))]
pub struct GdtPointer {
    /// Size of the DT.
    pub limit: u16,
    /// Pointer to the memory region containing the DT.
    pub base: *const GdtProtectedMode,
}

unsafe impl Send for GdtPointer {}
unsafe impl Sync for GdtPointer {}

pub fn enter_protected_mode_and_jump_to_stage_3(entry_point: u16) {
    GDT.clear_interrupts_and_load();
    set_protected_mode_bit();
    unsafe {
        asm!(
            // align the stack
            "and esp, 0xffffff00",
            // push arguments
            // push entry point address
            "push {entry_point:e}",
            entry_point = in(reg) entry_point as u32,
        );
        asm!(
            "ljmp $0x8, ${next_stage}",
            next_stage = const SECOND_STAGE_OFFSET, // Change this to the correct address
            options(att_syntax)
        );
    }
}

fn set_protected_mode_bit() -> u32 {
    let mut cr0: u32;
    unsafe {
        asm!("mov {:e}, cr0", out(reg) cr0, options(nomem, nostack, preserves_flags));
    }
    let cr0_protected = cr0 | 1;
    write_cr0(cr0_protected);
    cr0
}

fn write_cr0(val: u32) {
    unsafe { asm!("mov cr0, {:e}", in(reg) val, options(nostack, preserves_flags)) };
}