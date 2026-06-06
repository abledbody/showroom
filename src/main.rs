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
	vte::ansi::Rgb as AlacrittyColor,
};
use arboard::{Clipboard, GetExtLinux, SetExtLinux};
use eframe::{App, CreationContext, egui::{Event as EguiEvent, ViewportCommand}};
use egui_ratatui::RataguiBackend;
use ratatui::Terminal;
use soft_ratatui::{EmbeddedGraphics, SoftBackend, embedded_graphics_unicodefonts};
use termwiz::input::{KeyCodeEncodeModes, KeyboardEncoding};

use crate::translation::alacritty_clipboard_type_to_arboard_kind;

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
	ratagui_terminal: Terminal<RataguiBackend<EmbeddedGraphics>>,
	active_terminal: Arc<FairMutex<Term<EventProxy>>>,
	event_rx: Receiver<AlacrittyEvent>,
	event_tx: EventLoopSender,
	clipboard: Clipboard,
}

impl State {
	fn new(
		_creation_context: &CreationContext,
		ratagui_terminal: Terminal<RataguiBackend<EmbeddedGraphics>>,
		active_terminal: Arc<FairMutex<Term<EventProxy>>>,
		event_rx: Receiver<AlacrittyEvent>,
		event_tx: EventLoopSender,
		clipboard: Clipboard,
	) -> Self {
		State {
			ratagui_terminal,
			active_terminal,
			event_rx,
			event_tx,
			clipboard,
		}
	}

	fn apply_alacritty_event(&mut self, ctx: &eframe::egui::Context, event: AlacrittyEvent) -> Result<(), Box<dyn std::error::Error>> {
		Ok(match event {
			AlacrittyEvent::ColorRequest(index, fmt) => {
				let color = self.active_terminal.lock().colors()[index].unwrap_or(AlacrittyColor {
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
					cell_width: 8,
					cell_height: 13,
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
			AlacrittyEvent::Title(title) => ctx.send_viewport_cmd(ViewportCommand::Title(title)),
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
		match self
			.ratagui_terminal
			.draw(|f| translation::transfer_surface(&self.active_terminal, f.buffer_mut()))
		{
			Err(_) => eprintln!("Failed to draw terminal."),
			_ => {}
		};

		ui.add(self.ratagui_terminal.backend_mut());
	}

	fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
	let soft_backend = SoftBackend::new(
		DEFAULT_WIDTH,
		DEFAULT_HEIGHT,
		embedded_graphics_unicodefonts::MONO_8X13,
		Some(embedded_graphics_unicodefonts::MONO_8X13_BOLD),
		Some(embedded_graphics_unicodefonts::MONO_8X13_ITALIC),
	);

	let backend = RataguiBackend::new("Showroom", soft_backend);

	let config = AlacrittyConfig::default();
	let (event_tx, event_rx) = std::sync::mpsc::channel();
	let event_proxy = EventProxy(event_tx);
	let size = Size {
		total_lines: DEFAULT_TOTAL_LINES,
		screen_lines: DEFAULT_HEIGHT as usize,
		columns: DEFAULT_WIDTH as usize,
	};
	let active_terminal = Arc::new(FairMutex::new(Term::new(config, &size, event_proxy.clone())));

	let ratagui_terminal = Terminal::new(backend)?;

	let shell_path = match std::env::var_os("SHELL") {
		Some(shell_path) => match shell_path.into_string() {
			Ok(shell_path) => shell_path,
			Err(_) => "bash".into(),
		},
		None => "bash".into(),
	};

	tty::setup_env();
	let pty = tty::new(
		&Options {
			shell: Some(Shell::new(shell_path, vec![])),
			..Default::default()
		},
		WindowSize {
			num_lines: DEFAULT_HEIGHT,
			num_cols: DEFAULT_WIDTH,
			cell_width: 8,
			cell_height: 13,
		},
		0,
	)?;

	let event_loop = EventLoop::new(active_terminal.clone(), event_proxy, pty, false, false)?;
	let loop_tx = event_loop.channel();
	event_loop.spawn();

	eframe::run_native(
		"Showroom",
		eframe::NativeOptions::default(),
		Box::new(|cc| {
			Ok(Box::new(State::new(
				cc,
				ratagui_terminal,
				active_terminal,
				event_rx,
				loop_tx,
				Clipboard::new()?
			)))
		}),
	)?;

	Ok(())
}
