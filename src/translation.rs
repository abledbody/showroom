use arboard::LinuxClipboardKind;
use eframe::egui;
use alacritty_terminal::{term::{ClipboardType, color::Colors}, vte::ansi::{self, NamedColor}};
use egui::Color32;
use termwiz::input::KeyCode;


pub(crate) fn alacritty_to_egui_color(c: ansi::Color, colors: &Colors) -> Color32 {
	use alacritty_terminal::vte::ansi::Color::*;

	match c {
		Named(named_color) => {
			match colors[named_color as usize] {
				Some(rgb) => Color32::from_rgb(rgb.r, rgb.g, rgb.b),
				None => {
					match named_color {
						NamedColor::Background => Color32::from_rgb(30, 30, 30),
						_ => Color32::WHITE,
					}
				},
			}
		},
		Spec(rgb) => Color32::from_rgb(rgb.r, rgb.g, rgb.b),
		Indexed(index) => {
			if let Some(rgb) = colors[index as usize] {
				Color32::from_rgb(rgb.r, rgb.g, rgb.b)
			}
			else {
				eprintln!("Attempted to render color with index {}, which is unhandled. Rendering as magenta.", index);
				Color32::MAGENTA
			}
		},
	}
}

pub(crate) fn egui_key_to_code(key: eframe::egui::Key) -> Option<KeyCode> {
	use eframe::egui::Key;

	match key {
		Key::ArrowDown   => Some(KeyCode::DownArrow),
		Key::ArrowLeft   => Some(KeyCode::LeftArrow),
		Key::ArrowRight  => Some(KeyCode::RightArrow),
		Key::ArrowUp     => Some(KeyCode::UpArrow),

		Key::Escape      => Some(KeyCode::Escape),
		Key::Tab         => Some(KeyCode::Tab),
		Key::Backspace   => Some(KeyCode::Backspace),
		Key::Enter       => Some(KeyCode::Enter),
		Key::Insert      => Some(KeyCode::Insert),
		Key::Delete      => Some(KeyCode::Delete),
		Key::Home        => Some(KeyCode::Home),
		Key::End         => Some(KeyCode::End),
		Key::PageUp      => Some(KeyCode::PageUp),
		Key::PageDown    => Some(KeyCode::PageDown),
		Key::Copy        => Some(KeyCode::Copy),
		Key::Cut         => Some(KeyCode::Cut),
		Key::Paste       => Some(KeyCode::Paste),

		Key::F1          => Some(KeyCode::Function(1)),
		Key::F2          => Some(KeyCode::Function(2)),
		Key::F3          => Some(KeyCode::Function(3)),
		Key::F4          => Some(KeyCode::Function(4)),
		Key::F5          => Some(KeyCode::Function(5)),
		Key::F6          => Some(KeyCode::Function(6)),
		Key::F7          => Some(KeyCode::Function(7)),
		Key::F8          => Some(KeyCode::Function(8)),
		Key::F9          => Some(KeyCode::Function(9)),
		Key::F10         => Some(KeyCode::Function(10)),
		Key::F11         => Some(KeyCode::Function(11)),
		Key::F12         => Some(KeyCode::Function(12)),
		Key::F13         => Some(KeyCode::Function(13)),
		Key::F14         => Some(KeyCode::Function(14)),
		Key::F15         => Some(KeyCode::Function(15)),
		Key::F16         => Some(KeyCode::Function(16)),
		Key::F17         => Some(KeyCode::Function(17)),
		Key::F18         => Some(KeyCode::Function(18)),
		Key::F19         => Some(KeyCode::Function(19)),
		Key::F20         => Some(KeyCode::Function(20)),
		Key::F21         => Some(KeyCode::Function(21)),
		Key::F22         => Some(KeyCode::Function(22)),
		Key::F23         => Some(KeyCode::Function(23)),
		Key::F24         => Some(KeyCode::Function(24)),
		Key::F25         => Some(KeyCode::Function(25)),
		Key::F26         => Some(KeyCode::Function(26)),
		Key::F27         => Some(KeyCode::Function(27)),
		Key::F28         => Some(KeyCode::Function(28)),
		Key::F29         => Some(KeyCode::Function(29)),
		Key::F30         => Some(KeyCode::Function(30)),
		Key::F31         => Some(KeyCode::Function(31)),
		Key::F32         => Some(KeyCode::Function(32)),
		Key::F33         => Some(KeyCode::Function(33)),
		Key::F34         => Some(KeyCode::Function(34)),
		Key::F35         => Some(KeyCode::Function(35)),

		Key::BrowserBack => Some(KeyCode::BrowserBack),

		_                => None,
	}
}

pub(crate) fn egui_mod_to_termwiz(modifiers: egui::Modifiers) -> termwiz::input::Modifiers {
	use termwiz::input::Modifiers as TwMod;

	(if modifiers.ctrl || modifiers.command {TwMod::CTRL} else {TwMod::NONE})
		| if modifiers.alt {TwMod::ALT} else {TwMod::NONE}
		| if modifiers.shift {TwMod::SHIFT} else {TwMod::NONE}
		| if modifiers.mac_cmd {TwMod::SUPER} else {TwMod::NONE}
}

#[cfg(all(unix, not(any(target_os = "macos", target_os = "android", target_os = "emscripten"))))]
pub(crate) fn alacritty_clipboard_type_to_arboard_kind(t: ClipboardType) -> LinuxClipboardKind {
	match t {
		ClipboardType::Clipboard => LinuxClipboardKind::Clipboard,
		ClipboardType::Selection => LinuxClipboardKind::Primary,
	}
}