# Name-entry controls and directly painted shells

A manual MiniGoChi Rescue showed the name/gender screen with its background and
labels visible, while both text fields, the gender choice and the OK button were
missing. Replaying the recording reproduced that screen without a fatal error.
Temporary geometry logging established that the positioned children had sensible
rectangles, but their GFormComponent parent had a 0×0 rectangle. The controls
were therefore clipped out. This was not evidence of a stopped CPU.

## Shared fixes

- The default ShellComponent constructor now initializes display-sized geometry
  immediately. A guest may paint a shell through its own Card without calling
  ShellComponent.show(). The explicitly sized constructor retains its arguments.
- ShellComponent.addComponent overloads attach the work component and focus it,
  as does setWorkComponent. Previously only the latter connected cmpWork, so a
  normally added work component never received its shell layout.
- Container painting validates layout before invoking guest paintContent. Guest
  overrides that omit a superclass paint call must not skip layout validation.
- Buttons and text controls advertise input capability so form traversal can
  reach them. No key press or confirmation is manufactured.
- TextComponent now stores UTF-16 data and cursor/length state in guest fields,
  implements string assignment, insertion, deletion, length limits and input
  constraints, and paints text/cursors with password masking. TextFieldComponent
  initializes its supplied text/constraint and delegates insertion correctly.
- Fire opens a guest-owned full-screen editor; a subsequent paired Fire press
  and release returns to the existing Card. Editor keys are consumed instead of
  reaching the game below it. Basic lower-case Latin multi-tap and numeric input
  are available; the left softkey switches these modes, arrows move the cursor,
  and Clear/right softkey delete. Direct typed characters are supported too.
  The editor uses the existing guest clock and Card/event machinery, so its state
  and keys remain part of replay. Hangul keypad composition is now implemented; see [Korean keypad](korean-keypad.md)
  and [shared native input](native-korean-input.md) for coverage and controls.

The unrelated Component.configure stub was investigated but was not used on this
recorded path; its unconfirmed mask constants were not guessed or changed.
Neither game archives nor saves were patched, and no game/class-name condition
was added to shared emulation. CPU scheduling, timer delays and refresh behavior
were not changed. Ordinary checkpoint build guards remain enabled.

## Evidence and validation

The same Rescue prefix with the fixes paints both fields, gender choice and
confirmation button. Automated input using isolated copies of its initial saves
opens the editor, enters both names, changes gender and confirms the form. The
game reaches its tutorial and remains Running with 117 cumulative paints in that
bounded run. This count is not gameplay FPS or a full-game compatibility claim.

Regression tests cover constructor geometry before show, guest paint overrides,
work-component attachment and focus, explicit sizes, UTF-16 insert/delete ranges,
unchanged state after invalid input, length truncation, numeric restrictions,
password rendering, multi-tap input, release handling and editor return. Existing
WIPI Java, KTF and MIDP suites pass 115 tests.

Primary contract references:

- [WIPI TextComponent SDK](https://nikita36078.github.io/J2ME_Docs/docs/WIPI_API_1_1_1/org/kwis/msp/lwc/TextComponent.html)
- [WIPI TextFieldComponent SDK](https://nikita36078.github.io/J2ME_Docs/docs/WIPI_API_1_1_1/org/kwis/msp/lwc/TextFieldComponent.html)
- [WIPI ShellComponent SDK](https://nikita36078.github.io/J2ME_Docs/docs/WIPI_API_1_1_1/org/kwis/msp/lwc/ShellComponent.html)
- Korea Wireless Internet Standardization Forum, *Wireless Internet Platform for
  Interoperability 2.0.1*, Java API pages 300, 326–330
  ([manual mirror](https://manualzz.com/doc/13408437/korea-wireless-internet-standardization-forum-wireless-co...)).

Game-derived recordings, screenshots, disassembly and saves remain private test
artifacts. This work does not publish a release or update the Thor build.

The small-screen Master of Sword 2 and Fortune Golf 3D also pass their existing
bounded boot/input probes without detected errors; Golf's intentional first-start
exit and second-launch save initialization are preserved. Web/WASM compilation,
formatting and diff whitespace checks pass. Targeted Clippy completes with style
warnings. The ordinary, non-audit APK is installed with all 220 game archives
retained; private test saves were isolated from normal game data.
