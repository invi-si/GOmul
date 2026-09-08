//! Test-only entry points for benchmarking the full app library with dlopen.
//! Reuse the deterministic CPU workloads and their register/memory validation.

#[unsafe(no_mangle)]
pub extern "C" fn wie_bench_setup(workload: u32) -> u32 {
    wie_cpu_bench::bench_setup(workload)
}

#[unsafe(no_mangle)]
pub extern "C" fn wie_bench_run(batches: u32) -> u32 {
    wie_cpu_bench::bench_run(batches)
}

#[unsafe(no_mangle)]
pub extern "C" fn wie_bench_validate() -> u32 {
    wie_cpu_bench::bench_validate()
}

#[unsafe(no_mangle)]
pub extern "C" fn wie_bench_report_ptr() -> *const u8 {
    wie_cpu_bench::bench_report_ptr()
}

#[unsafe(no_mangle)]
pub extern "C" fn wie_bench_report_len() -> usize {
    wie_cpu_bench::bench_report_len()
}
