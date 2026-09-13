# Dragon Road: slow update rate caused by in-game speed setting

The reported low frame rate reproduced on the Android AVD on 2026-09-12, including the title/menu and new-game opening. GOmul was already configured at 2× speed.

The game’s own 환경설정 → 속도설정 displayed **Speed Off** (empty bar). Selecting **Speed 5** using the game’s controls removed the slow cadence. The normal, uninstrumented installed app changed from roughly 7–8 to 28–30 guest paints/second in menus at the same GOmul 2× setting. This is a configuration change, not an emulator CPU optimization or a claim of original-hardware timing accuracy. Guest paints are not measured physical presentations.

A separate diagnostic package reproduced the issue using copied private game data. Loss-free timer traces show the slow state alternating short and long delays, including requests over 400 guest milliseconds. The title callbacks were around 1–2 ms; a 5.59-second opening-story trace recorded 7.88 paints/sec, 183 ms active guest-CPU wall time and 160 ms active host-API wall time. Most elapsed time was therefore outside active execution. These instrumented wall measurements include observer/preemption costs and are attribution evidence, not optimization timings.

Independent inspection of the supplied guest binary found a ten-entry pacing table: 500, 160, 145, 130, 110, 100, 90, 70, 50, 30 ms. The game computes a bounded re-arm delay using its chosen entry and elapsed time. Its visible speed setting and the before/after behavior establish the practical remedy without patching guest code or shared timer semantics.

The initial full boot trace overflowed and was rejected. Later title census, short story trace and speed-setting census had zero lost records. Private binaries, saves, captures and screenshots remain outside the repository.

No emulator runtime, game archive, timer lifecycle, scheduler, rendering cap or GOmul speed setting was changed. The game wrote its own selected setting to persistent storage. Do not add a game-name speed override; users can choose the level that feels appropriate through the game’s existing menu.

Reopening the normal app’s Dragon Road entry preserved the setting; the title then reported 32–33 guest paints/sec at the unchanged 2× GOmul speed. The normal app was left on that title screen for manual play.

## Current local website verification

Fresh private saves on the current normal native web runtime reproduce `Speed Off` at GOmul 1×. In the same environment-settings menu, successive samples separated by a scripted 10-second wait count 249→287 paints (about 3.8 guest paints/sec). Using the game’s own right-arrow controls to select `Speed 5` changes the equivalent menu sample to 567→723 (about 15.6/sec). Sampling is approximate host-wall throughput, not physical presentation FPS. No profiling build or shared timing modification was used.

Closing and relaunching this private game runtime preserves `Speed 5`, verified by reopening the speed menu. The remedy is **환경설정 → 속도설정 → right arrow to Speed 5 → OK**. This test changes only the diagnostic private save, not any user’s browser save. Users can choose a different speed level to suit their preference.


## Current Android 1× verification

Fresh isolated save in the current normal Android APK: the same speed-settings screen records 177→215 paints over 10.008 seconds at Speed Off (3.80 guest paints/sec), then 258→416 over 10.007 seconds at Speed 5 (15.79/sec). Screenshots verify both settings. GOmul remains explicitly at 1×; only five guest Right inputs change the guest setting. This confirms the remedy on Android at normal emulator speed, without claiming physical presentation FPS or original-device accuracy. Existing user saves were untouched. Private evidence: additional-downloads/android-speed-comparison-36.
