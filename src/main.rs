use eframe::{App, CreationContext};
use egui_ratatui::RataguiBackend;
use ratatui::Terminal;
use soft_ratatui::{EmbeddedGraphics, SoftBackend, embedded_graphics_unicodefonts};

struct State {
	terminal: Terminal<RataguiBackend<EmbeddedGraphics>>,
}

impl State {
	fn new(
		_creation_context: &CreationContext,
		terminal: Terminal<RataguiBackend<EmbeddedGraphics>>,
	) -> Self {
		State { terminal }
	}
}

impl App for State {
	fn ui(&mut self, ui: &mut eframe::egui::Ui, _frame: &mut eframe::Frame) {
		ui.add(self.terminal.backend_mut());
	}
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
	let soft_backend = SoftBackend::new(
		80,
		32,
		embedded_graphics_unicodefonts::MONO_8X13,
		Some(embedded_graphics_unicodefonts::MONO_8X13_BOLD),
		Some(embedded_graphics_unicodefonts::MONO_8X13_ITALIC),
	);

	let backend = RataguiBackend::new("Showroom", soft_backend);

	let terminal = Terminal::new(backend)?;

	eframe::run_native(
		"Showroom",
		eframe::NativeOptions::default(),
		Box::new(|cc| Ok(Box::new(State::new(cc, terminal)))),
	)?;

	Ok(())
}
