/* Compile with the same NDK AArch64 clang used to link android_pc_sample:
 * aarch64-linux-android24-clang -fsyntax-only android_pc_layout.c
 * Independent ABI check: libc 0.2.189's Rust ucontext_t omits this padding.
 */
#include <stddef.h>
#include <sys/time.h>
#include <sys/ucontext.h>

_Static_assert(ITIMER_PROF == 2, "Unexpected profiling timer selector");
_Static_assert(offsetof(ucontext_t, uc_mcontext) == 176, "Unexpected ucontext layout");
_Static_assert(offsetof(mcontext_t, pc) == 264, "Unexpected mcontext PC layout");
_Static_assert(offsetof(ucontext_t, uc_mcontext) + offsetof(mcontext_t, pc) == 440,
               "Unexpected saved PC offset");
