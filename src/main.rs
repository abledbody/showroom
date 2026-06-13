use std::{
	borrow::Cow, default::Default, error::Error, sync::{
		Arc,
		mpsc::Receiver,
	}
};

use alacritty_terminal::{
	Term,
	event::{Event as AlacrittyEvent, EventListener, WindowSize},
	event_loop::{EventLoop, EventLoopSender, Msg as AlacrittyMsg},
	grid::Dimensions,
	sync::FairMutex,
	term::Config as AlacrittyConfig,
	tty::{self, Options, Shell},
	vte::ansi::{self, Rgb as AlacrittyColor},
};
use arboard::{Clipboard, GetExtLinux, SetExtLinux};
use eframe::{App, CreationContext, egui::{Event as EguiEvent, ViewportCommand}};
use termwiz::input::{KeyCodeEncodeModes, KeyboardEncoding};

use crate::translation::{alacritty_clipboard_type_to_arboard_kind, alacritty_to_egui_color};

mod translation;

const DEFAULT_TOTAL_LINES: usize = 4000;
const DEFAULT_WIDTH: u16 = 80;
const DEFAULT_HEIGHT: u16 = 32;
const CELL_WIDTH: u16 = 8;
const CELL_HEIGHT: u16 = 13;

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
	) -> Self {
		State {
			terminal,
			event_rx,
			event_tx,
			clipboard,
			cols,
			rows,
		}
	}

	fn apply_alacritty_event(&mut self, ctx: &eframe::egui::Context, event: AlacrittyEvent) -> Result<(), Box<dyn std::error::Error>> {
		eprintln!("{:?}", event);
		Ok(match event {
			AlacrittyEvent::ColorRequest(index, fmt) => {
				let color = self.terminal.lock().colors()[index].unwrap_or(AlacrittyColor {
					r: 0,
					g: 0,
					b: 0,
				});
				self.event_tx.send(AlacrittyMsg::Input(fmt(color).into_bytes().into()))?;
			}
			AlacrittyEvent::PtyWrite(text) => self.event_tx.send(AlacrittyMsg::Input(Cow::Owned(text.into_bytes())))?,
			AlacrittyEvent::TextAreaSizeRequest(fmt) => {
				let size = WindowSize {
					num_lines: DEFAULT_HEIGHT,
					num_cols: DEFAULT_WIDTH,
					cell_width: CELL_WIDTH,
					cell_height: CELL_HEIGHT,
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
}

impl App for State {
	fn ui(&mut self, ui: &mut eframe::egui::Ui, _frame: &mut eframe::Frame) {
		render_terminal(&self.terminal, ui);
	}

	fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
		let content_size = ctx.content_rect().size();
		let next_cols = (content_size.x / (CELL_WIDTH as f32)).ceil() as u16;
		let next_rows = (content_size.y / (CELL_HEIGHT as f32)).ceil() as u16;

		if next_cols != self.cols || next_rows != self.rows {
			if let Err(e) = self.event_tx.send(
				AlacrittyMsg::Resize(
					WindowSize {
						num_cols: next_cols,
						num_lines: next_rows,
						cell_width: CELL_WIDTH,
						cell_height: CELL_HEIGHT,
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

pub(crate) fn render_terminal(
	term: &Arc<FairMutex<Term<EventProxy>>>,
	ui: &mut egui::Ui,
) {
	let font_id = egui::FontId::monospace(13.0);

	let painter = ui.painter();
	let origin = ui.min_rect().min;

	let term = term.lock_unfair();
	let content = term.renderable_content();
	let alac_colors = content.colors;

	for cell in content.display_iter {
		let x = cell.point.column.0 as f32 * CELL_WIDTH as f32;
		let y = (cell.point.line.0 + content.display_offset as i32) as f32 * CELL_WIDTH as f32;
		let pos = origin + egui::vec2(x, y);
		let rect = egui::Rect::from_min_size(pos, egui::vec2(CELL_WIDTH as f32, CELL_HEIGHT as f32));

		let bg = alacritty_to_egui_color(cell.bg, &alac_colors);
		let fg = alacritty_to_egui_color(cell.fg, &alac_colors);

		painter.rect_filled(rect, 0.0, bg);
		if cell.c != ' ' {
			painter.text(pos, egui::Align2::LEFT_TOP, cell.c, font_id.clone(), fg);
		}
	}
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
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
			cell_width: CELL_WIDTH,
			cell_height: CELL_HEIGHT,
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

	let event_loop = EventLoop::new(active_terminal.clone(), event_proxy, pty, false, false)?;
	let loop_tx = event_loop.channel();
	event_loop.spawn();

	eframe::run_native(
		"Showroom",
		eframe::NativeOptions::default(),
		Box::new(|cc| {
			Ok(Box::new(State::new(
				cc,
				active_terminal,
				event_rx,
				loop_tx,
				Clipboard::new()?,
				DEFAULT_WIDTH,
				DEFAULT_HEIGHT,
			)))
		}),
	)?;

	Ok(())
}
