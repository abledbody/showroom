use std::{
	borrow::Cow, default::Default, error::Error, sync::{
		Arc,
		mpsc::Receiver,
	}
};

use alacritty_terminal::{
	Term, event::{
		Event as AlacrittyEvent,
		EventListener,
		WindowSize
	},
	event_loop::{
		EventLoop,
		EventLoopSender,
		Msg as AlacrittyMsg
	},
	grid::Dimensions,
	index::{Column, Line},
	sync::FairMutex,
	term::Config as AlacrittyConfig,
	tty::{self, Options, Shell},
	vte::ansi::{self, Rgb as AlacrittyColor}
};
use arboard::{Clipboard, GetExtLinux, SetExtLinux};
use eframe::{App, CreationContext, egui::{Event as EguiEvent, ViewportCommand}};
use egui::{TextFormat, text::LayoutJob};
use termwiz::input::{KeyCodeEncodeModes, KeyboardEncoding};

use crate::translation::{alacritty_clipboard_type_to_arboard_kind, alacritty_to_egui_color};

mod translation;

const DEFAULT_TOTAL_LINES: usize = 4000;
const DEFAULT_WIDTH: u16 = 80;
const DEFAULT_HEIGHT: u16 = 32;

const DEFAULT_KEY_CODE_ENCODE_MODE: KeyCodeEncodeModes = KeyCodeEncodeModes {
	encoding: KeyboardEncoding::Xterm,
	application_cursor_keys: false,
	newline_mode: false,
	modify_other_keys: None,
};

struct State {
	terminal: Arc<FairMutex<Term<EventProxy>>>,
	event_rx: Receiver<AlacrittyEvent>,
	event_tx: EventLoopSender,
	clipboard: Clipboard,
	cols: u16,
	rows: u16,
	cell_width: f32,
	cell_height: f32,
}

impl State {
	fn new(
		_creation_context: &CreationContext,
		terminal: Arc<FairMutex<Term<EventProxy>>>,
		event_rx: Receiver<AlacrittyEvent>,
		event_tx: EventLoopSender,
		clipboard: Clipboard,
		cols: u16,
		rows: u16,
		cell_width: f32,
		cell_height: f32,
	) -> Self {
		State {
			terminal,
			event_rx,
			event_tx,
			clipboard,
			cols,
			rows,
			cell_width,
			cell_height,
		}
	}

	fn apply_alacritty_event(&mut self, ctx: &eframe::egui::Context, event: AlacrittyEvent) -> Result<(), Box<dyn std::error::Error>> {
		Ok(match event {
			AlacrittyEvent::ColorRequest(index, fmt) => {
				let color = self.terminal.lock().colors()[index].unwrap_or(AlacrittyColor { r: 0, g: 0, b: 0, });
				self.event_tx.send(AlacrittyMsg::Input(fmt(color).into_bytes().into()))?;
			}
			AlacrittyEvent::PtyWrite(text) => self.event_tx.send(AlacrittyMsg::Input(Cow::Owned(text.into_bytes())))?,
			AlacrittyEvent::TextAreaSizeRequest(fmt) => {
				let size = WindowSize {
					num_lines: DEFAULT_HEIGHT,
					num_cols: DEFAULT_WIDTH,
					cell_width: self.cell_width.round() as u16,
					cell_height: self.cell_height.round() as u16,
				};
				self.event_tx.send(AlacrittyMsg::Input(Cow::Owned(fmt(size).into_bytes())))?;
			},
			AlacrittyEvent::Wakeup => {
				ctx.request_repaint()
			},
			AlacrittyEvent::ClipboardStore(clipboard_type, text) => {
				_ = self.clipboard.set().clipboard(alacritty_clipboard_type_to_arboard_kind(clipboard_type)).text(text)
			},
			AlacrittyEvent::ClipboardLoad(clipboard_type, receiver) => {
				if let Ok(text) = self.clipboard.get().clipboard(alacritty_clipboard_type_to_arboard_kind(clipboard_type)).text() {
					receiver(&text);
				}
			},
			AlacrittyEvent::Title(title) => ctx.send_viewport_cmd(ViewportCommand::Title(title + " — Showroom")),
			AlacrittyEvent::Exit => ctx.send_viewport_cmd(ViewportCommand::Close),
			_ => {}
		})
	}

	fn apply_egui_event(&mut self, e: EguiEvent) -> Result<(), Box<dyn Error>> {
		let string = match e {
			EguiEvent::Text(text) => text,
			EguiEvent::Key {
				key,
				physical_key: _,
				pressed,
				repeat: _,
				modifiers,
			} => match translation::egui_key_to_code(key) {
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

		self.event_tx.send(AlacrittyMsg::Input(Cow::Owned(string.into_bytes())))?;

		Ok(())
	}

	fn render_terminal(
		self: &Self,
		ui: &mut egui::Ui,
	) {
		let font_id = egui::FontId::monospace(13.0);

		let painter = ui.painter();
		let origin = ui.min_rect().min;

		let term = self.terminal.lock_unfair();
		let content = term.renderable_content();
		let alac_colors = content.colors;
		let grid = term.grid();

		let mut job = LayoutJob::default();

		for row_i in 0..grid.screen_lines() {
			let line = &grid[Line(row_i as i32 - content.display_offset as i32)];

			for col_i in 0..grid.columns() {
				let cell = &line[Column(col_i as usize)];

				let bg = alacritty_to_egui_color(cell.bg, &alac_colors);
				let fg = alacritty_to_egui_color(cell.fg, &alac_colors);

				job.append(&cell.c.to_string(), 0.0, TextFormat {
					font_id: font_id.clone(),
					color: fg,
					background: bg,
					..Default::default()
				});
			}

			if row_i != grid.screen_lines() - 1 {
				job.append("\n", 0.0, TextFormat::default())
			}
		}
		
		let galley = painter.layout_job(job);
		painter.galley(origin + egui::Vec2::ZERO, galley, egui::Color32::MAGENTA);
	}
}

impl App for State {
	fn ui(&mut self, ui: &mut eframe::egui::Ui, _frame: &mut eframe::Frame) {
		self.render_terminal(ui);
	}

	fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
		let font_id = egui::FontId::monospace(13.0);
		(self.cell_width, self.cell_height) = ctx.fonts_mut(|f| (f.glyph_width(&font_id, ' '), f.row_height(&font_id)));

		let content_size = ctx.content_rect().size();
		let next_cols = (content_size.x / self.cell_width).ceil() as u16;
		let next_rows = (content_size.y / self.cell_height).ceil() as u16;

		if next_cols != self.cols || next_rows != self.rows {
			if let Err(e) = self.event_tx.send(
				AlacrittyMsg::Resize(
					WindowSize {
						num_cols: next_cols,
						num_lines: next_rows,
						cell_width: self.cell_width.round() as u16,
						cell_height: self.cell_height.round() as u16,
					}
				)
			) {
				eprintln!("Failed to resize terminal: {}", e);
			}

			self.terminal.lock().resize(Size {
				total_lines: DEFAULT_TOTAL_LINES,
				screen_lines: next_rows as usize,
				columns: next_cols as usize,
			});

			self.cols = next_cols;
			self.rows = next_rows;
		}

		while let Ok(event) = self.event_rx.try_recv() {
			if let Err(e) = self.apply_alacritty_event(&ctx, event) {
				eprintln!("{}", e);
			}
		}

		let events = ctx.input(|i| i.events.clone());

		for e in events {
			if let Err(err) = self.apply_egui_event(e) {
				eprintln!("{}", err);
			}
		}
	}
}

#[derive(Clone)]
struct EventProxy(std::sync::mpsc::Sender<AlacrittyEvent>);

impl EventListener for EventProxy {
	fn send_event(&self, event: AlacrittyEvent) {
		let _ = self.0.send(event);
	}
}

struct Size {
	total_lines: usize,
	screen_lines: usize,
	columns: usize,
}

impl Dimensions for Size {
	fn total_lines(&self) -> usize {
		self.total_lines
	}

	fn screen_lines(&self) -> usize {
		self.screen_lines
	}

	fn columns(&self) -> usize {
		self.columns
	}
}

fn set_default_colors(term: &Arc<FairMutex<Term<EventProxy>>>) {
	use ansi::{Handler, NamedColor::*, Rgb};

	let mut term = term.lock();
	
	for &(index, rgb) in &[
		(Black         as usize, Rgb { r:  17, g:  24, b:  30 }),
		(Red           as usize, Rgb { r: 140, g:   0, b:   0 }),
		(Green         as usize, Rgb { r:   0, g: 160, b:  30 }),
		(Yellow        as usize, Rgb { r: 205, g: 205, b:   0 }),
		(Blue          as usize, Rgb { r:  70, g:  90, b: 255 }),
		(Magenta       as usize, Rgb { r: 205, g:   0, b: 205 }),
		(Cyan          as usize, Rgb { r:   0, g: 205, b: 205 }),
		(White         as usize, Rgb { r: 229, g: 229, b: 229 }),
		(BrightBlack   as usize, Rgb { r:  80, g:  90, b: 100 }),
		(BrightRed     as usize, Rgb { r: 255, g:  30, b:  50 }),
		(BrightGreen   as usize, Rgb { r:  80, g: 255, b:  40 }),
		(BrightYellow  as usize, Rgb { r: 255, g: 255, b:   0 }),
		(BrightBlue    as usize, Rgb { r: 140, g: 140, b: 255 }),
		(BrightMagenta as usize, Rgb { r: 255, g:  50, b: 240 }),
		(BrightCyan    as usize, Rgb { r:   0, g: 255, b: 255 }),
		(BrightWhite   as usize, Rgb { r: 255, g: 255, b: 255 }),
		(Foreground    as usize, Rgb { r: 229, g: 229, b: 229 }),
		(Background    as usize, Rgb { r:  17, g:  24, b:  30 }),
		(Cursor        as usize, Rgb { r: 229, g: 229, b: 229 }),
	] {
		term.set_color(index, rgb);
	}
}

fn main() -> Result<(), Box<dyn std::error::Error>> {


	eframe::run_native(
		"Showroom",
		eframe::NativeOptions::default(),
		Box::new(|ctx| {
			let shell_path = match std::env::var_os("SHELL") {
				Some(shell_path) => match shell_path.into_string() {
					Ok(shell_path) => shell_path,
					Err(_) => "bash".into(),
				},
				None => "bash".into(),
			};

			tty::setup_env();
			if std::process::Command::new("infocmp")
				.arg("alacritty")
				.output()
				.map(|o| !o.status.success())
				.unwrap_or(true)
			{
				unsafe { std::env::set_var("TERM", "xterm-256color"); }
			}

			let pty = tty::new(
				&Options {
					shell: Some(Shell::new(shell_path, vec![])),
					..Default::default()
				},
				WindowSize {
					num_lines: DEFAULT_HEIGHT,
					num_cols: DEFAULT_WIDTH,
					cell_width: 12,
					cell_height: 15,
				},
				0,
			)?;

			let config = AlacrittyConfig::default();
			let (event_tx, event_rx) = std::sync::mpsc::channel();
			let event_proxy = EventProxy(event_tx);
			let size = Size {
				total_lines: DEFAULT_TOTAL_LINES,
				screen_lines: DEFAULT_HEIGHT as usize,
				columns: DEFAULT_WIDTH as usize,
			};
			let active_terminal = Arc::new(FairMutex::new(Term::new(config, &size, event_proxy.clone())));

			set_default_colors(&active_terminal);

			let event_loop = EventLoop::new(active_terminal.clone(), event_proxy, pty, false, false)?;
			let loop_tx = event_loop.channel();
			event_loop.spawn();

			Ok(Box::new(State::new(
				ctx,
				active_terminal,
				event_rx,
				loop_tx,
				Clipboard::new()?,
				DEFAULT_WIDTH,
				DEFAULT_HEIGHT,
				0.0,
				0.0,
			)))
		}),
	)?;

	Ok(())
}
