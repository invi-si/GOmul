# WIPI heap reporting

`MC_knlGetTotalMemory` and `MC_knlGetFreeMemory` previously returned a constant
1 MiB. They now use the context's allocator-backed capacity and availability
in both LGT and KTF.

Total capacity excludes bucket bitmap metadata, unused bucket-region slack,
and the initial list header/canary. Free capacity counts unallocated bucket
slots and free list runs, accounting for coalescing adjacent free blocks without
mutating them. It is aggregate capacity, not the largest contiguous allocation;
bucket size classes and fragmentation can still prevent a particular allocation.
No heap size or allocation policy changes were made.

Queries read guest metadata; no host counters need restoration during replay
or Quick Load. Invalid list headers return an error instead of looping.
Regression coverage checks both allocation pools, size-class rounding, free
and coalescing behavior, and corrupt headers. Targeted ARM/LGT/KTF/WIPI C tests
pass (194 tests).
