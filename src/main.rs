use std::error::Error;

use eframe::{App, CreationContext};
use egui_ratatui::RataguiBackend;
use ratatui::Terminal;
use shadow_terminal::{
	active_terminal::ActiveTerminal, output::native::{CompleteSurface, Output as TerminalOutput, SurfaceDiff}, shadow_terminal::Config as ShadowTermConfig, termwiz::{
		input::{KeyCodeEncodeModes, KeyboardEncoding},
		surface::Surface,
	}
};
use soft_ratatui::{EmbeddedGraphics, SoftBackend, embedded_graphics_unicodefonts};

mod translation;

const DEFAULT_WIDTH: u16 = 80;
const DEFAULT_HEIGHT: u16 = 32;

const DEFAULT_KEY_CODE_ENCODE_MODE: KeyCodeEncodeModes = KeyCodeEncodeModes {
	encoding: KeyboardEncoding::Xterm,
	application_cursor_keys: false,
	newline_mode: false,
	modify_other_keys: None,
};

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
		let string = match e {
			eframe::egui::Event::Text(text) => text,
			eframe::egui::Event::Key {
				key,
				physical_key: _,
				pressed,
				repeat: _,
				modifiers,
			} => match translation::egui_key_to_termwiz_keycode(key) {
				Some(keycode) => keycode.encode(
					translation::egui_mod_to_termwiz(modifiers),
					DEFAULT_KEY_CODE_ENCODE_MODE,
					pressed,
				)?,
				None => "".to_string(),
			},
			_ => "".to_string(),
		};

		if string.len() == 0 {
			return Ok(());
		}

		for slice in string.as_bytes().chunks(128) {
			let mut bytes = [0u8; 128];
			bytes[..slice.len()].copy_from_slice(slice);
			self.active_terminal.pty_input_tx.try_send(bytes)?
		}

		Ok(())
	}
}

impl App for State {
	fn ui(&mut self, ui: &mut eframe::egui::Ui, _frame: &mut eframe::Frame) {
		match self
			.ratagui_terminal
			.draw(|f| translation::transfer_surface(&mut self.surface, f.buffer_mut()))
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
		let shell = std::env::var_os("SHELL").unwrap_or_else(|| "bash".into());
		ActiveTerminal::start(ShadowTermConfig {
			command: vec![shell],
			width: DEFAULT_WIDTH,
			height: DEFAULT_HEIGHT,
			..Default::default()
		})
	});

	let ratagui_terminal = Terminal::new(backend)?;

	eframe::run_native(
		"Showroom",
		eframe::NativeOptions::default(),
		Box::new(|cc| Ok(Box::new(State::new(cc, ratagui_terminal, active_terminal)))),
	)?;

	Ok(())
}
