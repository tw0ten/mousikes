mod config;

use rand::Rng;
use ratatui::{
	crossterm::{
		event::{self, Event, KeyCode},
		terminal, ExecutableCommand,
	},
	layout::{Constraint, Layout},
	prelude::CrosstermBackend,
	Frame, Terminal,
};
use rodio::{source::SineWave, Decoder, OutputStreamBuilder, Sink, Source};
use std::{
	env, fs,
	io::{self, stdout},
	path::Path,
	time::Duration,
};

fn main() -> io::Result<()> {
	let mut stream_handle = OutputStreamBuilder::open_default_stream().unwrap();
	stream_handle.log_on_drop(false);
	let sink = &Sink::connect_new(&stream_handle.mixer());
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

	terminal::enable_raw_mode()?;
	stdout().execute(terminal::EnterAlternateScreen)?;
	let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

	let mut rng = rand::rng();

	let location = env::current_dir()
		.unwrap_or(".".into())
		.display()
		.to_string();

	let mut s = String::new();

	let mut t = (String::new(), Duration::ZERO);

	loop {
		if !sink.is_paused() && sink.empty() {
			let files = ls(".");
			if files.len() > 0 {
				t = e(&files[rng.random_range(0..files.len())], sink).unwrap_or(t)
			}
		}

		terminal.draw(|frame: &mut Frame| {
			use ratatui::{
				style::{Style, Stylize},
				widgets::{self, LineGauge},
			};
			use Constraint::{Length, Min};
			let [title_area, _main_area, status_area] =
				Layout::vertical([Length(2), Min(0), Length(1)]).areas(frame.area());
			{
				let silence_break = sink.len() == 1;
				let progress = sink.get_pos();
				if !silence_break {
					frame.render_widget(
						LineGauge::default()
							.label("")
							.ratio(if t.1.is_zero() {
								0.0
							} else {
								progress.as_secs_f64() / t.1.as_secs_f64()
							})
							.filled_style(Style::new().white())
							.style(Style::new().black()),
						status_area,
					);
				}
				{
					let search = s != "";
					let s = if !search { &t.0 } else { &c(&s) };
					frame.render_widget(
						widgets::Paragraph::new(format!(
							"[{}] {}{}\n{} <{}> ({})",
							&location,
							if search { "/" } else { "" },
							if !search && silence_break { "" } else { s },
							match sink.is_paused() {
								true => "=",
								_ => match sink.empty() {
									false => "+",
									_ => "-",
								},
							},
							sink.volume(),
							if silence_break {
								format!("-")
							} else {
								format!("{}/{}", progress.as_secs(), t.1.as_secs())
							},
						)),
						title_area,
					);
				}
			}
		})?;

		if event::poll(config::INTERVAL)? {
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
								t = e(&s, sink).unwrap_or(t);
								s.clear();
							}
						},

						KeyCode::Tab => {
							sink.clear();
							sink.play()
						}

						KeyCode::Up => {
							sink.set_volume(1f32.min(sink.volume() + config::VOLUME_CHANGE))
						}
						KeyCode::Down => {
							sink.set_volume(0f32.max(sink.volume() - config::VOLUME_CHANGE))
						}

						KeyCode::Left => match sink.is_paused() {
							false => sink.pause(),
							_ => _ = sink.try_seek(sink.get_pos().saturating_sub(config::SEEK)),
						},
						KeyCode::Right => match sink.is_paused() {
							true => sink.play(),
							_ => _ = sink.try_seek(sink.get_pos().saturating_add(config::SEEK)),
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

	terminal::disable_raw_mode()?;
	stdout().execute(terminal::LeaveAlternateScreen)?;
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
	assert!(o.len() < 2_usize.pow(12));
	o.sort_by(|a, b| a.len().cmp(&b.len()));
	o
}

fn c(s: &String) -> String {
	for f in ls(".") {
		if f.contains(s) {
			return f;
		}
	}
	s.to_string()
}

fn e(s: &String, sink: &Sink) -> Option<(String, Duration)> {
	let s = c(s);
	let p = Path::new(&s);
	if let Ok(v) = fs::File::open(p) {
		if let Ok(v) = Decoder::try_from(v) {
			let duration = v.total_duration();
			sink.clear();
			sink.append(v);
			sink.append(
				SineWave::new(1.0)
					.take_duration(config::SILENCE_PADDING)
					.amplify(0.0),
			);
			_ = sink.try_seek(Duration::ZERO);
			sink.play();
			return Some((
				p.to_string_lossy().to_string(),
				duration.unwrap_or(Duration::ZERO),
			));
		}
	}
	None
}
