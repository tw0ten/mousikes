mod config;

use config::*;
use rand::Rng;
use ratatui::{
	crossterm::{
		event::{self, Event, KeyCode},
		terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
		ExecutableCommand,
	},
	prelude::CrosstermBackend,
	widgets::Paragraph,
	Frame, Terminal,
};
use rodio::{Decoder, OutputStream, Sink};
use std::{
	env,
	fs::{self, File},
	io::{self, stdout, BufReader},
	path::Path,
	time::Duration,
};

fn main() -> io::Result<()> {
	let (_stream, stream_handle) = OutputStream::try_default().unwrap();
	let sink = &Sink::try_new(&stream_handle).unwrap();
	sink.set_volume(0.0);
	sink.play();

	{
		let mut p = String::new();
		for a in env::args().skip(1) {
			match p.as_str() {
				"-v" | "--volume" => {
					sink.set_volume(1f32.min(0f32.max(a.parse().unwrap_or(sink.volume()))))
				}
				_ => match a.as_str() {
					"-p" | "--pause" => sink.pause(),
					"-h" | "--help" => {
						println!("mousikes");
						println!("\t-v | --volume <f32>");
						println!("\t-p | --pause");
						return Ok(());
					}
					_ => {
						p = a;
						continue;
					}
				},
			}
			p.clear();
		}
	}

	enable_raw_mode()?;
	stdout().execute(EnterAlternateScreen)?;
	let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

	let mut rng = rand::thread_rng();

	let location = env::current_dir()
		.unwrap_or(".".into())
		.display()
		.to_string();

	let (mut s, mut t) = (String::new(), String::new());

	loop {
		if !sink.is_paused() && sink.empty() {
			let files = ls(".");
			if files.len() > 0 {
				t = e(&files[rng.gen_range(0..files.len())], &t, sink)
			}
		}

		terminal.draw(|frame: &mut Frame| {
			let s = if s == "" { &t } else { &c(&s) };
			frame.render_widget(
				Paragraph::new(format!(
					"{{{}}}\n{} <{}> ({})\n[{}]",
					s,
					match sink.is_paused() {
						true => "=",
						_ => match sink.empty() {
							false => "+",
							_ => "-",
						},
					},
					sink.volume(),
					sink.get_pos().as_secs(),
					&location,
				)),
				frame.area(),
			);
		})?;

		if event::poll(INTERVAL)? {
			match event::read()? {
				Event::Key(key) => match key.kind {
					event::KeyEventKind::Press => match key.code {
						KeyCode::Esc => break,

						KeyCode::Enter => match s.as_str() {
							"" => match sink.is_paused() {
								true => sink.play(),
								_ => sink.pause(),
							},
							_ => {
								t = e(&s, &t, sink);
								s.clear();
							}
						},
						KeyCode::Tab => {
							sink.clear();
							sink.play()
						}

						KeyCode::Up => sink.set_volume(1f32.min(sink.volume() + VOLUME_CHANGE)),
						KeyCode::Down => sink.set_volume(0f32.max(sink.volume() - VOLUME_CHANGE)),

						KeyCode::Left => match sink.is_paused() {
							false => sink.pause(),
							_ => _ = sink.try_seek(sink.get_pos().saturating_sub(SEEK)),
						},
						KeyCode::Right => match sink.is_paused() {
							true => sink.play(),
							_ => _ = sink.try_seek(sink.get_pos().saturating_add(SEEK)),
						},

						KeyCode::Backspace => s.clear(),
						KeyCode::Char(v) => s.push(v),
						_ => {}
					},
					_ => {}
				},
				Event::Paste(v) => s.push_str(&v),
				_ => {}
			}
		}
	}

	disable_raw_mode()?;
	stdout().execute(LeaveAlternateScreen)?;
	Ok(())
}

fn ls(dir: &str) -> Vec<String> {
	let mut o: Vec<String> = Vec::new();
	if let Ok(v) = fs::read_dir(dir) {
		for v in v.flatten() {
			let v = v.path();
			let s = v.to_string_lossy();
			match v.is_dir() {
				false => o.push(s[2..].to_string()),
				_ => o.extend(ls(&s)),
			}
		}
	}
	o.sort();
	o
}

fn c(s: &String) -> String {
	for f in ls(".") {
		if f.starts_with(s) {
			return f;
		}
	}
	s.to_string()
}

fn e(s: &String, t: &String, sink: &Sink) -> String {
	let s = c(s);
	let p = Path::new(&s);
	if let Ok(v) = File::open(p) {
		if let Ok(v) = Decoder::new(BufReader::new(v)) {
			sink.clear();
			_ = sink.try_seek(Duration::ZERO);
			sink.append(v);
			sink.play();
			return p.to_string_lossy().to_string();
		}
	}
	t.to_string()
}
