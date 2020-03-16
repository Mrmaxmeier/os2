use crate::sched::user::SavedRegs;
use alloc::string::String;
use alloc::vec::Vec;

// Hash for a 4k range
struct PageHash(u64);

struct Mapping {
    perm_r: bool,
    perm_w: bool,
    perm_x: bool,
    start_address: u64,
    size: u64,
    hint: String,
    pages: Vec<PageHash>,
}

struct SnapshotDef {
    regs: SavedRegs,
    mappings: Vec<Mapping>,
}
