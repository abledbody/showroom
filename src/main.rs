use eframe::{App, CreationContext};
use egui_ratatui::RataguiBackend;
use ratatui::Terminal;
use shadow_terminal::{
	active_terminal::ActiveTerminal,
	output::native::{CompleteSurface, Output as TerminalOutput, SurfaceDiff},
	termwiz::{self, color::ColorAttribute, surface::Surface},
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
}

impl App for State {
	fn ui(&mut self, ui: &mut eframe::egui::Ui, _frame: &mut eframe::Frame) {
		match self
			.ratagui_terminal
			.draw(|f| transfer_surface(&mut self.surface, f.buffer_mut()))
		{
			Err(_) => panic!("Failed to draw terminal."),
			_ => {}
		};

		ui.add(self.ratagui_terminal.backend_mut());
	}

	fn update(&mut self, _ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
		while let Ok(output) = self.active_terminal.surface_output_rx.try_recv() {
			self.sync_surface(output);
		}
	}
}

fn transfer_surface(surface: &mut Surface, buf: &mut ratatui::buffer::Buffer) {
	for (y, line) in surface.screen_lines().iter().enumerate() {
		for cell in line.visible_cells() {
			let x = cell.cell_index() as u16;
			let y = y as u16;

			let attrs = cell.attrs();
			let style = ratatui::style::Style::new()
				.fg(termwiz_color_to_ratatui(attrs.foreground()))
				.bg(termwiz_color_to_ratatui(attrs.background()));

			if x < buf.area.width && y < buf.area.height {
				buf[(x as u16, y as u16)]
					.set_symbol(cell.str())
					.set_style(style);
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

	let ratagui_terminal = Terminal::new(backend)?;

	let rt = tokio::runtime::Runtime::new().unwrap();
	let active_terminal = rt.block_on(async {
		ActiveTerminal::start(shadow_terminal::shadow_terminal::Config::default())
	});

	eframe::run_native(
		"Showroom",
		eframe::NativeOptions::default(),
		Box::new(|cc| Ok(Box::new(State::new(cc, ratagui_terminal, active_terminal)))),
	)?;

	Ok(())
}
