use std::error::Error;

use eframe::{App, CreationContext, egui};
use egui_ratatui::RataguiBackend;
use ratatui::Terminal;
use shadow_terminal::{
	active_terminal::ActiveTerminal,
	output::native::{CompleteSurface, Output as TerminalOutput, SurfaceDiff},
	termwiz::{self, color::ColorAttribute, input::{KeyCodeEncodeModes, KeyboardEncoding}, surface::Surface},
	wezterm_term::KeyCode,
};
use soft_ratatui::{EmbeddedGraphics, SoftBackend, embedded_graphics_unicodefonts};

const DEFAULT_WIDTH: u16 = 80;
const DEFAULT_HEIGHT: u16 = 32;

struct State {
	ratagui_terminal: Terminal<RataguiBackend<EmbeddedGraphics>>,
	active_terminal: ActiveTerminal,
	surface: Surface,
}

impl State {
	fn new(
		_creation_context: &CreationContext,
		ratagui_terminal: Terminal<RataguiBackend<EmbeddedGraphics>>,
		active_terminal: ActiveTerminal,
	) -> Self {
		State {
			ratagui_terminal,
			active_terminal,
			surface: Surface::new(DEFAULT_WIDTH as usize, DEFAULT_HEIGHT as usize),
		}
	}

	fn sync_surface(&mut self, output: TerminalOutput) {
		match output {
			TerminalOutput::Diff(diff) => {
				self.surface.add_changes(match diff {
					SurfaceDiff::Scrollback(scrollback_diff) => scrollback_diff.changes,
					SurfaceDiff::Screen(screen_diff) => screen_diff.changes,
					_ => todo!(),
				});
			}
			TerminalOutput::Complete(complete) => {
				self.surface = match complete {
					CompleteSurface::Scrollback(complete_scrollback) => complete_scrollback.surface,
					CompleteSurface::Screen(complete_screen) => complete_screen.surface,
					_ => todo!(),
				};
			}
			_ => todo!(),
		}
	}

	fn apply_egui_event(&mut self, e: eframe::egui::Event) -> Result<(), Box<dyn Error>> {
		match e {
			eframe::egui::Event::Text(text) => {
				for slice in text.as_bytes().chunks(128) {
					let mut bytes = [0u8; 128];
					bytes[..slice.len()].copy_from_slice(slice);
					self.active_terminal.pty_input_tx.try_send(bytes)?
				}
			}
			eframe::egui::Event::Key {
				key,
				physical_key: _,
				pressed,
				repeat: _,
				modifiers,
			} => {
				if let Some(keycode) = egui_key_to_termwiz_keycode(key) {
					keycode.encode(
						egui_mod_to_termwiz(modifiers),
						KeyCodeEncodeModes {
							encoding: KeyboardEncoding::Xterm,
							application_cursor_keys: false,
							newline_mode: false,
							modify_other_keys: None,
						},
						pressed,
					)?;
				}
			}
			_ => {}
		}

		Ok(())
	}
}

fn egui_key_to_termwiz_keycode(key: eframe::egui::Key) -> Option<KeyCode> {
	match key {
		eframe::egui::Key::ArrowDown => Some(KeyCode::DownArrow),
		eframe::egui::Key::ArrowLeft => Some(KeyCode::LeftArrow),
		eframe::egui::Key::ArrowRight => Some(KeyCode::RightArrow),
		eframe::egui::Key::ArrowUp => Some(KeyCode::UpArrow),
		eframe::egui::Key::Escape => Some(KeyCode::Escape),
		eframe::egui::Key::Tab => Some(KeyCode::Tab),
		eframe::egui::Key::Backspace => Some(KeyCode::Backspace),
		eframe::egui::Key::Enter => Some(KeyCode::Enter),
		eframe::egui::Key::Insert => Some(KeyCode::Insert),
		eframe::egui::Key::Delete => Some(KeyCode::Delete),
		eframe::egui::Key::Home => Some(KeyCode::Home),
		eframe::egui::Key::End => Some(KeyCode::End),
		eframe::egui::Key::PageUp => Some(KeyCode::PageUp),
		eframe::egui::Key::PageDown => Some(KeyCode::PageDown),
		eframe::egui::Key::Copy => Some(KeyCode::Copy),
		eframe::egui::Key::Cut => Some(KeyCode::Cut),
		eframe::egui::Key::Paste => Some(KeyCode::Paste),
		eframe::egui::Key::F1 => Some(KeyCode::Function(1)),
		eframe::egui::Key::F2 => Some(KeyCode::Function(2)),
		eframe::egui::Key::F3 => Some(KeyCode::Function(3)),
		eframe::egui::Key::F4 => Some(KeyCode::Function(4)),
		eframe::egui::Key::F5 => Some(KeyCode::Function(5)),
		eframe::egui::Key::F6 => Some(KeyCode::Function(6)),
		eframe::egui::Key::F7 => Some(KeyCode::Function(7)),
		eframe::egui::Key::F8 => Some(KeyCode::Function(8)),
		eframe::egui::Key::F9 => Some(KeyCode::Function(9)),
		eframe::egui::Key::F10 => Some(KeyCode::Function(10)),
		eframe::egui::Key::F11 => Some(KeyCode::Function(11)),
		eframe::egui::Key::F12 => Some(KeyCode::Function(12)),
		eframe::egui::Key::F13 => Some(KeyCode::Function(13)),
		eframe::egui::Key::F14 => Some(KeyCode::Function(14)),
		eframe::egui::Key::F15 => Some(KeyCode::Function(15)),
		eframe::egui::Key::F16 => Some(KeyCode::Function(16)),
		eframe::egui::Key::F17 => Some(KeyCode::Function(17)),
		eframe::egui::Key::F18 => Some(KeyCode::Function(18)),
		eframe::egui::Key::F19 => Some(KeyCode::Function(19)),
		eframe::egui::Key::F20 => Some(KeyCode::Function(20)),
		eframe::egui::Key::F21 => Some(KeyCode::Function(21)),
		eframe::egui::Key::F22 => Some(KeyCode::Function(22)),
		eframe::egui::Key::F23 => Some(KeyCode::Function(23)),
		eframe::egui::Key::F24 => Some(KeyCode::Function(24)),
		eframe::egui::Key::F25 => Some(KeyCode::Function(25)),
		eframe::egui::Key::F26 => Some(KeyCode::Function(26)),
		eframe::egui::Key::F27 => Some(KeyCode::Function(27)),
		eframe::egui::Key::F28 => Some(KeyCode::Function(28)),
		eframe::egui::Key::F29 => Some(KeyCode::Function(29)),
		eframe::egui::Key::F30 => Some(KeyCode::Function(30)),
		eframe::egui::Key::F31 => Some(KeyCode::Function(31)),
		eframe::egui::Key::F32 => Some(KeyCode::Function(32)),
		eframe::egui::Key::F33 => Some(KeyCode::Function(33)),
		eframe::egui::Key::F34 => Some(KeyCode::Function(34)),
		eframe::egui::Key::F35 => Some(KeyCode::Function(35)),
		eframe::egui::Key::BrowserBack => Some(KeyCode::BrowserBack),
		_ => None,
	}
}

fn egui_mod_to_termwiz(modifiers: egui::Modifiers) -> termwiz::input::Modifiers {
	use termwiz::input::Modifiers as TwMod;
	(if modifiers.ctrl || modifiers.command {
		TwMod::CTRL
	} else {
		TwMod::NONE
	}) | if modifiers.alt {
		TwMod::ALT
	} else {
		TwMod::NONE
	} | if modifiers.shift {
		TwMod::SHIFT
	} else {
		TwMod::NONE
	} | if modifiers.mac_cmd {
		TwMod::SUPER
	} else {
		TwMod::NONE
	}
}

impl App for State {
	fn ui(&mut self, ui: &mut eframe::egui::Ui, _frame: &mut eframe::Frame) {
		match self
			.ratagui_terminal
			.draw(|f| transfer_surface(&mut self.surface, f.buffer_mut()))
		{
			Err(_) => eprintln!("Failed to draw terminal."),
			_ => {}
		};

		ui.add(self.ratagui_terminal.backend_mut());
	}

	fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
		let mut redraw = false;

		while let Ok(output) = self.active_terminal.surface_output_rx.try_recv() {
			self.sync_surface(output);
			redraw = true;
		}

		if redraw {
			ctx.request_repaint();
		}

		let events = ctx.input(|i| i.events.clone());

		for e in events {
			if let Err(err) = self.apply_egui_event(e) {
				eprintln!("{}", err);
			}
		}
	}
}

fn transfer_surface(surface: &mut Surface, buf: &mut ratatui::buffer::Buffer) {
	for (y, line) in surface.screen_lines().iter().enumerate() {
		for cell in line.visible_cells() {
			let (x, y) = (cell.cell_index() as u16, y as u16);

			let attrs = cell.attrs();
			let style = ratatui::style::Style::new()
				.fg(termwiz_color_to_ratatui(attrs.foreground()))
				.bg(termwiz_color_to_ratatui(attrs.background()));

			if x < buf.area.width && y < buf.area.height {
				buf[(x, y)].set_symbol(cell.str()).set_style(style);
			}
		}
	}
}

fn termwiz_color_to_ratatui(c: ColorAttribute) -> ratatui::style::Color {
	use ratatui::style::Color;
	use termwiz::color::ColorAttribute::*;
	match c {
		Default => Color::Reset,
		PaletteIndex(i) => Color::Indexed(i),
		TrueColorWithDefaultFallback(s) | TrueColorWithPaletteFallback(s, _) => {
			let (r, g, b, _) = s.as_rgba_u8();
			Color::Rgb(r, g, b)
		}
	}
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
	let soft_backend = SoftBackend::new(
		DEFAULT_WIDTH,
		DEFAULT_HEIGHT,
		embedded_graphics_unicodefonts::MONO_8X13,
		Some(embedded_graphics_unicodefonts::MONO_8X13_BOLD),
		Some(embedded_graphics_unicodefonts::MONO_8X13_ITALIC),
	);

	let backend = RataguiBackend::new("Showroom", soft_backend);

	let rt = tokio::runtime::Runtime::new().unwrap();
	let active_terminal = rt.block_on(async {
		ActiveTerminal::start(shadow_terminal::shadow_terminal::Config::default())
	});

	let ratagui_terminal = Terminal::new(backend)?;

	eframe::run_native(
		"Showroom",
		eframe::NativeOptions::default(),
		Box::new(|cc| Ok(Box::new(State::new(cc, ratagui_terminal, active_terminal)))),
	)?;

	Ok(())
}
