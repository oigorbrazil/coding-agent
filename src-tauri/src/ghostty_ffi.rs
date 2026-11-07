//! FFI para o shim C do libghostty (`ghostty_shim.m`).
//!

//!

#![cfg(all(target_os = "macos", ghostty_linked))]

use std::os::raw::{c_char, c_void};

pub type CodingAgentSurface = *mut c_void;

#[allow(dead_code)]
extern "C" {
    pub fn coding_agent_ghostty_ensure_app() -> bool;
    pub fn coding_agent_ghostty_surface_new(
        nsview: *mut c_void,
        cwd: *const c_char,
        command: *const c_char,
        scale_factor: f64,
    ) -> CodingAgentSurface;
    pub fn coding_agent_ghostty_surface_set_frame(surface: CodingAgentSurface, x: f64, y: f64, w: f64, h: f64);
    pub fn coding_agent_ghostty_surface_set_hidden(surface: CodingAgentSurface, hidden: bool);
    pub fn coding_agent_ghostty_surface_set_size(surface: CodingAgentSurface, width_px: u32, height_px: u32);
    pub fn coding_agent_ghostty_surface_set_content_scale(surface: CodingAgentSurface, x: f64, y: f64);
    pub fn coding_agent_ghostty_surface_set_focus(surface: CodingAgentSurface, focused: bool);
    pub fn coding_agent_ghostty_surface_process_exited(surface: CodingAgentSurface) -> bool;
    pub fn coding_agent_ghostty_surface_draw(surface: CodingAgentSurface);
    pub fn coding_agent_ghostty_surface_free(surface: CodingAgentSurface);
    pub fn coding_agent_ghostty_app_tick();
    pub fn coding_agent_ghostty_kill_all();
    pub fn coding_agent_ghostty_surface_send_text(
        surface: CodingAgentSurface,
        utf8: *const c_char,
        len: usize,
    );
    pub fn coding_agent_ghostty_surface_read_screen(
        surface: CodingAgentSurface,
        out: *mut c_char,
        cap: usize,
    ) -> usize;
    pub fn coding_agent_ghostty_draw_count() -> u64;
    pub fn coding_agent_ghostty_test_ime_compose(
        surface: CodingAgentSurface,
        marked: *const c_char,
        final_: *const c_char,
    ) -> bool;
    pub fn coding_agent_ghostty_test_type_key(
        surface: CodingAgentSurface,
        characters: *const c_char,
        keycode: u16,
    ) -> bool;
    pub fn coding_agent_ghostty_test_last_key_text() -> *const c_char;
    pub fn coding_agent_ghostty_test_last_key_composing() -> bool;
}
