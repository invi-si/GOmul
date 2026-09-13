# Music Factory startup enumeration

Rescue 1789025710674 fails at LGT WIPI C service 1100 (0x44c). Native startup supplies a byte buffer and capacity, then walks NUL-separated names, including comparisons with SMSDATA, MMSDATA and CALLHISTORY. Supply an empty terminated list for the emulator’s absent phone data stores. The original API name is not confirmed. Invalid output/capacity returns -22; memory faults remain errors.

After this fix, continuing the usage screen reached missing service 410 (0x19a). This caller passes "tbl/B", a byte buffer, its capacity, and flag 1. It walks NUL-separated three-digit directory names and opens song resources beneath them. Implement the observed directory-list mode from actual classpath archive entries, deduplicating immediate children in lexical order. Read ZIP entry names without decompressing payloads. Other flags return -22 until their contracts are established. This currently enumerates packaged resources, not downloaded filesystem content.

Tests cover buffer sentinel preservation and invalid arguments for phone-store enumeration, and immediate-child extraction, deduplication and prefix boundaries for archive directories. No game-name conditions, save resets or guest timing changes.

Validation: 40 LGT unit tests and one integration test pass. Regular APK progresses through usage notice and publisher logo into the introductory character dialogue. Left the game there for manual tutorial testing. No claim of full-game compatibility.
