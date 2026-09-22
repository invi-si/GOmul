export interface GameInputSink {
  key_down(key: string): void;
  key_up(key: string): void;
}

const KEY_MAP: Record<string, string> = {
  Digit0: "1",
  Minus: "2",
  Equal: "3",
  KeyO: "4",
  KeyP: "5",
  BracketLeft: "6",
  KeyL: "7",
  Semicolon: "8",
  Quote: "9",
  Comma: "*",
  Period: "0",
  Slash: "#",
  Digit1: "1",
  Digit2: "2",
  Digit3: "3",
  Digit4: "4",
  Digit5: "5",
  Digit6: "6",
  Digit7: "7",
  Digit8: "8",
  Digit9: "9",
  Numpad0: "0",
  Numpad1: "1",
  Numpad2: "2",
  Numpad3: "3",
  Numpad4: "4",
  Numpad5: "5",
  Numpad6: "6",
  Numpad7: "7",
  Numpad8: "8",
  Numpad9: "9",
  NumpadMultiply: "*",
  KeyQ: "4",
  KeyW: "UP",
  KeyE: "6",
  KeyA: "LEFT",
  KeyS: "DOWN",
  KeyD: "RIGHT",
  KeyZ: "*",
  KeyX: "0",
  KeyC: "#",
  Backspace: "CLR",
  ArrowUp: "UP",
  ArrowLeft: "LEFT",
  ArrowRight: "RIGHT",
  ArrowDown: "DOWN",
  Space: "OK",
  Enter: "OK",
  NumpadEnter: "OK",
  ShiftLeft: "LSOFT",
  ShiftRight: "RSOFT",
  F1: "CALL",
  F2: "HANGUP",
};

export const keyboardGameKey = (event: Pick<KeyboardEvent, "code" | "key">): string | undefined => {
  if (event.key === "*" || event.key === "#") {
    return event.key;
  }
  return KEY_MAP[event.code];
};

// Multiple fingers, or a finger and a keyboard key, can hold the same phone key.
// Only the first press and final release should reach the emulator.
export class HeldGameKeys {
  private readonly sources = new Map<string, string>();
  private readonly counts = new Map<string, number>();
  private readonly sink: GameInputSink;

  constructor(sink: GameInputSink) {
    this.sink = sink;
  }

  press(source: string, key: string): void {
    if (this.sources.get(source) === key) {
      return;
    }
    this.release(source);
    this.sources.set(source, key);
    const count = this.counts.get(key) ?? 0;
    this.counts.set(key, count + 1);
    if (count === 0) {
      this.sink.key_down(key);
    }
  }

  release(source: string): boolean {
    const key = this.sources.get(source);
    if (key === undefined) {
      return false;
    }
    this.sources.delete(source);
    const count = this.counts.get(key)!;
    if (count === 1) {
      this.counts.delete(key);
      this.sink.key_up(key);
    } else {
      this.counts.set(key, count - 1);
    }
    return true;
  }

  releaseAll(): void {
    const keys = [...this.counts.keys()];
    this.sources.clear();
    this.counts.clear();
    for (const key of keys) {
      this.sink.key_up(key);
    }
  }
}

type RapidFireMode = "off" | "manual" | "auto";

class RapidFireKeys implements GameInputSink {
  mode: RapidFireMode = "off";
  private autoKey?: string;
  private readonly pulses = new Map<string, { down: boolean; timer: number }>();
  private readonly sink: GameInputSink;
  private readonly window: Window;

  constructor(sink: GameInputSink, window: Window) {
    this.sink = sink;
    this.window = window;
  }

  private start(key: string): void {
    if (this.pulses.has(key)) return;
    this.sink.key_down(key);
    const pulse = { down: true, timer: 0 };
    const tick = () => {
      if (this.pulses.get(key) !== pulse) return;
      pulse.down = !pulse.down;
      if (pulse.down) this.sink.key_down(key); else this.sink.key_up(key);
      pulse.timer = this.window.setTimeout(tick, 50);
    };
    this.pulses.set(key, pulse);
    pulse.timer = this.window.setTimeout(tick, 50);
  }

  private stop(key: string): void {
    const pulse = this.pulses.get(key);
    if (!pulse) return;
    this.window.clearTimeout(pulse.timer);
    this.pulses.delete(key);
    if (pulse.down) this.sink.key_up(key);
  }

  key_down(key: string): void {
    if (this.mode === "off" || (key !== "5" && key !== "OK")) {
      this.sink.key_down(key);
    } else if (this.mode === "manual") {
      this.start(key);
    } else if (this.autoKey === key) {
      this.stop(key);
      this.autoKey = undefined;
    } else {
      if (this.autoKey) this.stop(this.autoKey);
      this.autoKey = key;
      this.start(key);
    }
  }

  key_up(key: string): void {
    if (this.mode === "off" || (key !== "5" && key !== "OK")) this.sink.key_up(key);
    else if (this.mode === "manual") this.stop(key);
  }

  reset(): void {
    for (const key of this.pulses.keys()) this.stop(key);
    this.autoKey = undefined;
    this.mode = "off";
  }
}

export const bindGameInput = (root: HTMLElement, sink: GameInputSink) => {
  const document = root.ownerDocument;
  const window = document.defaultView!;
  const abortController = new AbortController();
  const options = { signal: abortController.signal };
  const buttons = [...root.querySelectorAll<HTMLButtonElement>("button[data-key]")];
  const rapidToggle = root.querySelector<HTMLButtonElement>("button[data-rapid-fire]");
  const activationTimers = new Map<string, number>();
  const showPressed = (key: string, pressed: boolean) => {
    for (const button of buttons) {
      if (button.dataset.key === key) {
        button.classList.toggle("is-pressed", pressed);
      }
    }
  };
  const rapid = new RapidFireKeys({
    key_down(key) {
      sink.key_down(key);
      showPressed(key, true);
    },
    key_up(key) {
      sink.key_up(key);
      showPressed(key, false);
    },
  }, window);
  const held = new HeldGameKeys(rapid);
  const showRapidMode = () => {
    if (!rapidToggle) return;
    const name = {off: "끔", manual: "수동", auto: "자동"}[rapid.mode];
    rapidToggle.textContent = `연사 ${name}`;
    rapidToggle.setAttribute("aria-label", `연사 ${name}. 숫자 5 또는 확인 키에 적용`);
    rapidToggle.setAttribute("aria-pressed", String(rapid.mode !== "off"));
    rapidToggle.title = rapid.mode === "manual" ? "5 또는 확인을 누르는 동안 연사" : rapid.mode === "auto" ? "5 또는 확인을 누르면 시작, 다시 누르면 중지" : "연사 끔";
  };
  showRapidMode();

  for (const button of buttons) {
    button.addEventListener("pointerdown", (event) => {
      if (event.button !== 0) {
        return;
      }
      event.preventDefault();
      button.setPointerCapture(event.pointerId);
      held.press(`pointer:${event.pointerId}`, button.dataset.key!);
    }, options);
    const release = (event: PointerEvent) => {
      event.preventDefault();
      held.release(`pointer:${event.pointerId}`);
    };
    button.addEventListener("pointerup", release, options);
    button.addEventListener("pointercancel", release, options);
    button.addEventListener("lostpointercapture", release, options);
    button.addEventListener("contextmenu", (event) => event.preventDefault(), options);
    button.addEventListener("click", (event) => {
      // Assistive technologies activate buttons with a click rather than pointer
      // events. Pointer clicks are already handled by the held-key path above.
      if (event.detail !== 0) {
        return;
      }
      const key = button.dataset.key!;
      const source = `activation:${key}`;
      const previousTimer = activationTimers.get(source);
      if (previousTimer !== undefined) {
        window.clearTimeout(previousTimer);
        held.release(source);
      }
      held.press(source, key);
      activationTimers.set(source, window.setTimeout(() => {
        activationTimers.delete(source);
        held.release(source);
      }, 100));
    }, options);
  }

  const isEditing = (target: EventTarget | null) =>
    target instanceof Element && target.closest("dialog, input, textarea, select, [contenteditable]:not([contenteditable=false])") !== null;

  document.addEventListener("keydown", (event) => {
    if (isEditing(event.target) || event.ctrlKey || event.metaKey || event.altKey) {
      return;
    }
    let key = keyboardGameKey(event);
    const button = event.target instanceof Element ? event.target.closest<HTMLButtonElement>("button") : null;
    if (button && (event.code === "Space" || event.code === "Enter" || event.code === "NumpadEnter")) {
      if (!buttons.includes(button)) {
        return;
      }
      key = button.dataset.key;
    }
    if (key) {
      event.preventDefault();
      if (!event.repeat) {
        held.press(`keyboard:${event.code}`, key);
      }
    }
  }, options);
  document.addEventListener("keyup", (event) => {
    // Release even if focus or modifiers changed since keydown.
    if (held.release(`keyboard:${event.code}`)) {
      event.preventDefault();
    }
  }, options);

  const releaseAll = () => {
    for (const timer of activationTimers.values()) {
      window.clearTimeout(timer);
    }
    activationTimers.clear();
    held.releaseAll();
    rapid.reset();
    showRapidMode();
  };
  rapidToggle?.addEventListener("click", () => {
    const next = rapid.mode === "off" ? "manual" : rapid.mode === "manual" ? "auto" : "off";
    releaseAll();
    rapid.mode = next;
    showRapidMode();
  }, options);
  document.addEventListener("focusin", (event) => {
    if (isEditing(event.target)) {
      releaseAll();
    }
  }, options);
  document.addEventListener("visibilitychange", () => {
    if (document.hidden) {
      releaseAll();
    }
  }, options);
  window.addEventListener("blur", releaseAll, options);
  window.addEventListener("pagehide", releaseAll, options);

  return {
    releaseAll,
    dispose() {
      releaseAll();
      abortController.abort();
    },
  };
};

export function directionalGameKey(key:string,numeric:boolean):string {
  return numeric?({UP:"2",LEFT:"4",RIGHT:"6",DOWN:"8"}[key]??key):key;
}
