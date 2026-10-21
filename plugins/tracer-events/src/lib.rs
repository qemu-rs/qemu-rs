use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use typed_builder::TypedBuilder;

#[derive(TypedBuilder, Clone, Debug, Default, Deserialize, Serialize)]
pub struct InstructionEvent {
    pub vaddr: u64,
    pub haddr: u64,
    pub disas: String,
    pub symbol: Option<String>,
    pub data: Vec<u8>,
}

#[derive(TypedBuilder, Clone, Debug, Default, Deserialize, Serialize)]
pub struct MemoryEvent {
    pub vaddr: u64,
    pub haddr: Option<u64>,
    pub haddr_is_io: Option<bool>,
    pub haddr_device_name: Option<String>,
    pub size_shift: usize,
    pub size_bytes: usize,
    pub sign_extended: bool,
    pub is_store: bool,
    pub big_endian: bool,
}

#[derive(TypedBuilder, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct SyscallSource {
    plugin_id: u64,
    vcpu_index: u32,
}

#[derive(TypedBuilder, Clone, Debug, Default, Deserialize, Serialize)]
pub struct SyscallEvent {
    pub num: i64,
    pub return_value: i64,
    pub args: [u64; 8],
    #[builder(default)]
    pub buffers: HashMap<usize, Vec<u8>>,
}

#[derive(TypedBuilder, Clone, Debug, Default, Deserialize, Serialize)]
pub struct Interrupt {
    pub vaddr: u64,
    pub handler: u64,
}

#[derive(TypedBuilder, Clone, Debug, Default, Deserialize, Serialize)]
pub struct Exception {
    pub epc: u64,
    pub handler: u64,
}

#[derive(TypedBuilder, Clone, Debug, Default, Deserialize, Serialize)]
pub struct HostCall {
    pub vaddr: u64,
    pub next: u64,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Registers(pub HashMap<String, Vec<u8>>);

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Event {
    Instruction {
        event: InstructionEvent,
        registers: Registers,
    },
    Memory(MemoryEvent),
    Syscall(SyscallEvent),
    Interrupt(Interrupt),
    Exception(Exception),
    HostCall(HostCall),
}
