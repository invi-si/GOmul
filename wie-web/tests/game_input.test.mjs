import assert from "node:assert/strict";
import test from "node:test";

import { bindGameInput, HeldGameKeys, keyboardGameKey, directionalGameKey } from "../src/ts/game_input.ts";

const recordingSink = () => {
  const events = [];
  return {
    events,
    key_down: (key) => events.push(["down", key]),
    key_up: (key) => events.push(["up", key]),
  };
};

// Small event fixtures exercise lifecycle wiring without a browser dependency.
// Device pointer capture and layout still need a real browser/device check.
class FakeElement extends EventTarget {
  editing = false;
  isButton = false;
  capturedPointers = [];
  pressed = new Set();
  classList = {
    toggle: (name, enabled) => enabled ? this.pressed.add(name) : this.pressed.delete(name),
  };
  closest(selector) {
    if (selector === "button") {
      return this.isButton ? this : null;
    }
    return this.editing ? this : null;
  }
  setPointerCapture(pointerId) { this.capturedPointers.push(pointerId); }
}
globalThis.Element = FakeElement;

const fire = (target, type, properties = {}) => {
  const event = new Event(type, { cancelable: true });
  for (const [key, value] of Object.entries(properties)) {
    Object.defineProperty(event, key, { value });
  }
  target.dispatchEvent(event);
  return event;
};

const fixture = () => {
  const document = new EventTarget();
  document.defaultView = new EventTarget();
  const timers = new Map();
  let nextTimer = 0;
  document.defaultView.setTimeout = (callback) => {
    const timer = ++nextTimer;
    timers.set(timer, callback);
    return timer;
  };
  document.defaultView.clearTimeout = (timer) => timers.delete(timer);
  const flushTimers = () => {
    for (const [timer, callback] of timers) {
      timers.delete(timer);
      callback();
    }
  };
  document.hidden = false;
  const buttons = ["UP", "OK", "0"].map((key) => {
    const button = new FakeElement();
    button.isButton = true;
    button.dataset = { key };
    return button;
  });
  const root = new FakeElement();
  root.ownerDocument = document;
  root.querySelectorAll = () => buttons;
  const sink = recordingSink();
  const input = bindGameInput(root, sink);
  return { document, window: document.defaultView, buttons, sink, input, timers, flushTimers };
};

test("one input source cannot release another source holding the same phone key", () => {
  const sink = recordingSink();
  const held = new HeldGameKeys(sink);
  held.press("keyboard:Enter", "OK");
  held.press("pointer:1", "OK");
  held.press("pointer:2", "OK");
  held.press("keyboard:Enter", "OK");
  held.release("pointer:1");
  held.release("keyboard:Enter");
  assert.deepEqual(sink.events, [["down", "OK"]]);
  held.release("pointer:2");
  held.release("pointer:2");
  assert.deepEqual(sink.events, [["down", "OK"], ["up", "OK"]]);
});

test("a reused source releases its previous key and releaseAll is idempotent", () => {
  const sink = recordingSink();
  const held = new HeldGameKeys(sink);
  held.press("pointer:1", "LEFT");
  held.press("pointer:1", "RIGHT");
  held.press("pointer:2", "OK");
  held.releaseAll();
  held.releaseAll();
  assert.deepEqual(sink.events, [
    ["down", "LEFT"], ["up", "LEFT"], ["down", "RIGHT"],
    ["down", "OK"], ["up", "RIGHT"], ["up", "OK"],
  ]);
});

test("directions and actions can be held together; cancellation and lost capture release independently", () => {
  const { buttons: [up, ok], sink, input } = fixture();
  fire(up, "pointerdown", { pointerId: 11, button: 0 });
  fire(ok, "pointerdown", { pointerId: 12, button: 0 });
  assert.deepEqual(up.capturedPointers, [11]);
  assert.deepEqual(ok.capturedPointers, [12]);
  assert(up.pressed.has("is-pressed"));
  fire(up, "pointercancel", { pointerId: 11 });
  assert(!up.pressed.has("is-pressed"));
  assert(ok.pressed.has("is-pressed"));
  fire(up, "lostpointercapture", { pointerId: 11 });
  fire(ok, "lostpointercapture", { pointerId: 12 });
  fire(ok, "pointerup", { pointerId: 12 });
  assert.deepEqual(sink.events, [["down", "UP"], ["down", "OK"], ["up", "UP"], ["up", "OK"]]);
  input.dispose();
});

test("keyboard and touch share a held key; repeat events do not create additional presses", () => {
  const { document, buttons: [, ok], sink, input } = fixture();
  fire(document, "keydown", { code: "Enter", key: "Enter", repeat: false });
  fire(document, "keydown", { code: "Enter", key: "Enter", repeat: true });
  fire(ok, "pointerdown", { pointerId: 1, button: 0 });
  fire(document, "keyup", { code: "Enter", key: "Enter" });
  assert.deepEqual(sink.events, [["down", "OK"]]);
  fire(ok, "pointerup", { pointerId: 1 });
  assert.deepEqual(sink.events, [["down", "OK"], ["up", "OK"]]);
  input.dispose();
});

test("releasing after focus or modifier changes cannot leave a keyboard key held", () => {
  const { document, sink, input } = fixture();
  const editor = new FakeElement();
  editor.editing = true;
  fire(document, "keydown", { code: "Digit3", key: "#" });
  fire(document, "keyup", { code: "Digit3", key: "3", target: editor });
  fire(document, "keydown", { code: "KeyW", key: "w", target: editor });
  fire(document, "keydown", { code: "KeyW", key: "w", metaKey: true });
  assert.deepEqual(sink.events, [["down", "#"], ["up", "#"]]);
  input.dispose();
});

for (const interruption of ["blur", "hidden", "pagehide", "editing", "dispose"]) {
  test(`${interruption} releases every held key exactly once`, () => {
    const { document, window, buttons: [up], sink, input } = fixture();
    fire(up, "pointerdown", { pointerId: 1, button: 0 });
    fire(document, "keydown", { code: "Space", key: " " });
    if (interruption === "hidden") {
      document.hidden = true;
      fire(document, "visibilitychange");
    } else if (interruption === "editing") {
      const editor = new FakeElement();
      editor.editing = true;
      fire(document, "focusin", { target: editor });
    } else if (interruption === "dispose") {
      input.dispose();
    } else {
      fire(window, interruption);
    }
    fire(up, "pointerup", { pointerId: 1 });
    fire(document, "keyup", { code: "Space", key: " " });
    input.dispose();
    fire(document, "keydown", { code: "Space", key: " " });
    fire(up, "pointerdown", { pointerId: 2, button: 0 });
    assert.deepEqual(sink.events, [["down", "UP"], ["down", "OK"], ["up", "UP"], ["up", "OK"]]);
  });
}

test("keyboard preserves numpad and other phone keys alongside the requested layout", () => {
  for (let digit = 0; digit <= 9; digit++) {
    assert.equal(keyboardGameKey({ code: `Digit${digit}`, key: `${digit}` }), digit === 0 ? "1" : `${digit}`);
    assert.equal(keyboardGameKey({ code: `Numpad${digit}`, key: `${digit}` }), `${digit}`);
  }
  const expected = {
    KeyQ: "4", KeyW: "UP", KeyE: "6", KeyA: "LEFT", KeyS: "DOWN", KeyD: "RIGHT",
    ArrowUp: "UP", ArrowLeft: "LEFT", ArrowDown: "DOWN", ArrowRight: "RIGHT",
    KeyZ: "*", KeyX: "0", KeyC: "#", Space: "OK", Enter: "OK", NumpadEnter: "OK",
    F1: "CALL", F2: "HANGUP", ShiftLeft: "LSOFT", ShiftRight: "RSOFT", Backspace: "CLR",
  };
  for (const [code, key] of Object.entries(expected)) {
    assert.equal(keyboardGameKey({ code, key: "" }), key);
  }
  assert.equal(keyboardGameKey({ code: "Digit8", key: "*" }), "*");
  assert.equal(keyboardGameKey({ code: "Escape", key: "Escape" }), undefined);
});

test("secondary mouse clicks do not press game keys and long-press menus are suppressed", () => {
  const { buttons: [up], sink, input } = fixture();
  fire(up, "pointerdown", { pointerId: 1, button: 2 });
  assert.deepEqual(sink.events, []);
  assert(fire(up, "contextmenu").defaultPrevented);
  input.dispose();
});

test("Space and Enter preserve native activation of toolbar buttons", () => {
  const { document, sink, input } = fixture();
  const toolbarButton = new FakeElement();
  toolbarButton.isButton = true;
  for (const code of ["Space", "Enter", "NumpadEnter"]) {
    const event = fire(document, "keydown", { code, key: "", target: toolbarButton });
    assert(!event.defaultPrevented);
    fire(document, "keyup", { code, key: "", target: toolbarButton });
  }
  assert.deepEqual(sink.events, []);
  input.dispose();
});

test("Space and Enter activate the focused phone button instead of always sending OK", () => {
  const { document, buttons: [up, , zero], sink, input } = fixture();
  for (const [code, target] of [["Space", up], ["Enter", zero]]) {
    assert(fire(document, "keydown", { code, key: "", target }).defaultPrevented);
    assert(fire(document, "keyup", { code, key: "", target }).defaultPrevented);
  }
  assert.deepEqual(sink.events, [["down", "UP"], ["up", "UP"], ["down", "0"], ["up", "0"]]);
  input.dispose();
});

test("assistive click activates a phone key without duplicating pointer clicks", () => {
  const { buttons: [up], sink, input, flushTimers } = fixture();
  fire(up, "click", { detail: 0 });
  assert.deepEqual(sink.events, [["down", "UP"]]);
  flushTimers();
  fire(up, "pointerdown", { pointerId: 1, button: 0 });
  fire(up, "pointerup", { pointerId: 1 });
  fire(up, "click", { detail: 1 });
  flushTimers();
  assert.deepEqual(sink.events, [["down", "UP"], ["up", "UP"], ["down", "UP"], ["up", "UP"]]);
  input.dispose();
});

test("assistive activation cannot release an independently held key and disposal cancels its timer", () => {
  const { document, buttons: [up], sink, input, timers, flushTimers } = fixture();
  fire(document, "keydown", { code: "ArrowUp", key: "ArrowUp" });
  fire(up, "click", { detail: 0 });
  flushTimers();
  assert.deepEqual(sink.events, [["down", "UP"]]);
  fire(up, "click", { detail: 0 });
  assert.equal(timers.size, 1);
  input.dispose();
  assert.equal(timers.size, 0);
  assert.deepEqual(sink.events, [["down", "UP"], ["up", "UP"]]);
});


test("The shared keyboard maps the requested physical key grid without changing other keys", () => {
  const rows = [
    [["KeyW", "KeyA", "KeyS", "KeyD"], ["UP", "LEFT", "DOWN", "RIGHT"]],
    [["Digit0", "Minus", "Equal"], ["1", "2", "3"]],
    [["KeyO", "KeyP", "BracketLeft"], ["4", "5", "6"]],
    [["KeyL", "Semicolon", "Quote"], ["7", "8", "9"]],
    [["Comma", "Period", "Slash"], ["*", "0", "#"]],
  ];
  for (const [codes, expected] of rows) {
    assert.deepEqual(codes.map(code => keyboardGameKey({code, key:""})), expected);
  }
  assert.equal(keyboardGameKey({code:"KeyO", key:"ㅐ"}), "4");
  assert.equal(keyboardGameKey({code:"Digit3", key:"#"}), "#");
  assert.equal(keyboardGameKey({code:"KeyW", key:"w"}), "UP");
});

test("Movement and keypad holds route independently and release on blur", () => {
  const f = fixture();
  fire(f.document, "keydown", {code:"KeyW", key:"w"});
  fire(f.document, "keydown", {code:"Semicolon", key:";"});
  fire(f.document, "keydown", {code:"KeyW", key:"w", repeat:true});
  fire(f.document, "keyup", {code:"KeyW", key:"w"});
  fire(f.window, "blur");
  assert.deepEqual(f.sink.events, [["down","UP"],["down","8"],["up","UP"],["up","8"]]);
  f.input.dispose();
});


test("numeric direction profiles preserve aliases until both inputs release",()=>{
 const sink=recordingSink(),held=new HeldGameKeys(sink);
 held.press("UP",directionalGameKey("UP",true));held.press("2",directionalGameKey("2",true));
 held.release("UP");assert.deepEqual(sink.events,[["down","2"]]);held.release("2");
 assert.deepEqual(sink.events,[["down","2"],["up","2"]]);
 for(const [key,value] of Object.entries({UP:"2",LEFT:"4",RIGHT:"6",DOWN:"8",OK:"OK",LSOFT:"LSOFT"})){
  assert.equal(directionalGameKey(key,true),value);assert.equal(directionalGameKey(key,false),key);
 }
});
