//! Diagnostics only. Host timestamps are observations, never guest time inputs.
use alloc::string::ToString;
use wasm_bindgen::prelude::*;
use wie_core_arm::{ArmCore, cpu_profile::ProfileMode};

#[wasm_bindgen(inline_js = "export function wieProfileNowNs() { return performance.now() * 1000000; }")]
extern "C" {
    #[wasm_bindgen(js_name = wieProfileNowNs)]
    fn now_ns_js() -> f64;
}

pub fn now_ns() -> u64 {
    now_ns_js() as u64
}

pub struct WebCpuProfile {
    pub core: Option<ArmCore>,
    mode: ProfileMode,
    pub update_checks: u64,
    pub redraw_events: u64,
    pub key_down_events: u64,
    pub key_up_events: u64,
    pub key_repeat_events: u64,
    event_checks_ns: u64,
    clock_regressions: u64,
}

impl WebCpuProfile {
    pub fn new(core: Option<ArmCore>) -> Self {
        Self {
            core,
            mode: ProfileMode::Off,
            update_checks: 0,
            redraw_events: 0,
            key_down_events: 0,
            key_up_events: 0,
            key_repeat_events: 0,
            event_checks_ns: 0,
            clock_regressions: 0,
        }
    }

    pub fn enabled(&self) -> bool {
        self.mode != ProfileMode::Off
    }

    pub fn begin_events(&mut self) -> Option<u64> {
        self.update_checks += u64::from(self.enabled());
        (self.mode == ProfileMode::Sampled).then(now_ns)
    }

    pub fn end_events(&mut self, started: Option<u64>) {
        if let Some(started) = started {
            let ended = now_ns();
            self.event_checks_ns += ended.saturating_sub(started);
            self.clock_regressions += u64::from(ended < started);
        }
    }

    fn core(&self) -> Result<&ArmCore, JsError> {
        self.core
            .as_ref()
            .ok_or_else(|| JsError::new("CPU profiling is available for the LGT ARM interpreter in this diagnostic build"))
    }

    pub fn configure(&mut self, mode: &str, interval: u32, seed: u32) -> Result<(), JsError> {
        let mode = match mode {
            "off" => ProfileMode::Off,
            "counts" => ProfileMode::Counts,
            "sampled" => ProfileMode::Sampled,
            _ => return Err(JsError::new("Expected off, counts or sampled")),
        };
        self.core()?
            .set_cpu_profiling(mode, interval, seed, now_ns)
            .map_err(|e| JsError::new(&e.to_string()))?;
        self.mode = mode;
        self.reset()
    }

    pub fn reset(&mut self) -> Result<(), JsError> {
        self.core()?.reset_cpu_profiling().map_err(|e| JsError::new(&e.to_string()))?;
        let core = self.core.take();
        let mode = self.mode;
        *self = Self::new(core);
        self.mode = mode;
        Ok(())
    }

    pub fn snapshot(&self) -> Result<JsValue, JsError> {
        let snapshot = self.core()?.cpu_profiling_snapshot().map_err(|e| JsError::new(&e.to_string()))?;
        let value = serde_json::json!({
            "core": snapshot,
            "stage_names": wie_core_arm::cpu_profile::Stage::ALL.map(|stage| stage.name()),
            "web_events": {
                "update_checks": self.update_checks,
                "redraw_events": self.redraw_events,
                "key_down_events": self.key_down_events,
                "key_up_events": self.key_up_events,
                "key_repeat_events": self.key_repeat_events,
                "event_checks_inclusive_ns": self.event_checks_ns,
                "clock_regressions": self.clock_regressions,
            },
            "timing_note": "Sampled host observations include observer overhead. CPU stage inclusive scopes nest. Run and SVC poll scopes overlap; do not add them. Web event checks exclude emulator.tick."
        });
        js_sys::JSON::parse(&value.to_string()).map_err(|_| JsError::new("Could not serialize CPU profile"))
    }
}
