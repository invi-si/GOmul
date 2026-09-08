#![no_std]
extern crate alloc;

#[cfg(feature = "cpu-profiling")]
pub mod cpu_profile;

#[cfg(feature = "cpu-throughput")]
pub mod cpu_throughput;

mod allocator;
mod binary_patches;
mod context;
mod core;
mod engine;
#[cfg(feature = "cpu-replay")]
pub use engine::cpu_replay;
mod function;
pub mod stdlib;
mod thread;
mod thread_wrapper;

#[cfg(not(target_arch = "wasm32"))]
mod gdb;

pub type ThreadId = usize;

#[cfg(feature = "cpu-benchmark")]
pub use engine::{Arm32CpuEngine, ArmEngine, ArmRegister, EngineRunResult, MemoryPermission};

pub use self::{
    allocator::Allocator,
    binary_patches::install_binary_patches,
    context::ArmCoreContext,
    core::{ArmCore, RUN_FUNCTION_LR, RunFunctionResult},
    function::{EmulatedFunction, EmulatedFunctionParam, JumpTo, RegisteredFunction, RegisteredFunctionHolder, ResultWriter, SvcId},
};
