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
    path::{Path, PathBuf},
    time::Duration,
};

const IDLE_POLL: Duration = Duration::from_millis(5000);

const SEEK: Duration = Duration::from_millis(5000);

const VOLUME_MAX: f32 = 1.5;
const VOLUME_SCALE: f32 = 1000.0;
const VOLUME_CHANGE: i32 = 25;

fn main() -> io::Result<()> {
    let (_stream, stream_handle) = OutputStream::try_default().unwrap();
    let sink = &Sink::try_new(&stream_handle).unwrap();
    sink.set_volume(0.0);
    sink.play();

    {
        let args: Vec<String> = env::args().collect();
        let mut p = String::new();
        for a in args {
            let s = p;
            p = String::new();
            match s.as_str() {
                "-v" | "--volume" => {
                    sink.set_volume(VOLUME_MAX.min(0f32.max(a.parse().unwrap_or(sink.volume()))))
                }
                _ => match a.as_str() {
                    "-p" | "--pause" => sink.pause(),
                    "-h" | "--help" => {
                        println!("mousikes");
                        println!("\t-v | --volume <f32>");
                        println!("\t-p | --pause");
                        return Ok(());
                    }
                    _ => p = a,
                },
            }
        }
    }

    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    let mut rng = rand::thread_rng();

    let location: &str = match env::current_dir() {
        Ok(v) => &v.display().to_string(),
        _ => ".",
    };

    let (mut s, mut t) = (String::new(), String::new());
    let mut interval = IDLE_POLL;

    loop {
        if !sink.is_paused() && sink.empty() {
            let files = lsf();
            if files.len() > 0 {
                t = e(
                    &files[rng.gen_range(0..files.len())]
                        .file_name()
                        .unwrap()
                        .to_string_lossy()
                        .to_string(),
                    &t,
                    sink,
                );
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

        if event::poll(interval)? {
            match event::read()? {
                Event::Key(key) => match key.kind {
                    event::KeyEventKind::Press => match key.code {
                        KeyCode::Esc => break,
                        KeyCode::Up => sink.set_volume(
                            (((VOLUME_SCALE * VOLUME_MAX) as i32)
                                .min((sink.volume() * VOLUME_SCALE) as i32 + VOLUME_CHANGE)
                                as f32)
                                / VOLUME_SCALE,
                        ),
                        KeyCode::Down => sink.set_volume(
                            (((VOLUME_SCALE * 0.0) as i32)
                                .max((sink.volume() * VOLUME_SCALE) as i32 - VOLUME_CHANGE)
                                as f32)
                                / VOLUME_SCALE,
                        ),
                        KeyCode::Left => match sink.is_paused() {
                            false => sink.pause(),
                            _ => _ = sink.try_seek(sink.get_pos().saturating_sub(SEEK)),
                        },
                        KeyCode::Right => match sink.is_paused() {
                            true => sink.play(),
                            _ => _ = sink.try_seek(sink.get_pos().saturating_add(SEEK)),
                        },
                        KeyCode::Tab => {
                            sink.clear();
                            sink.play()
                        }
                        KeyCode::Enter => match s.as_str() {
                            "" => match sink.is_paused() {
                                true => sink.play(),
                                _ => sink.pause(),
                            },
                            _ => {
                                t = e(&s, &t, sink);
                                s = String::new()
                            }
                        },
                        KeyCode::Backspace => s = String::new(),
                        KeyCode::Char(v) => s.push_str(&String::from(v)),
                        _ => {}
                    },
                    _ => {}
                },
                Event::Paste(v) => s.push_str(&v),
                Event::FocusGained => interval = Duration::from_millis(50),
                Event::FocusLost => interval = IDLE_POLL,
                _ => {}
            }
        }
    }

    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}

fn lsf() -> Vec<PathBuf> {
    fs::read_dir(".")
        .unwrap()
        .map(|f| f.expect("").path())
        .filter(|f| !f.is_dir())
        .collect()
}

fn c(s: &String) -> String {
    let files = lsf();
    for f in files {
        let f = f.file_name().unwrap().to_string_lossy().to_string();
        if f.starts_with(s) {
            return f;
        }
    }
    s.to_string()
}

fn e(s: &String, t: &String, sink: &Sink) -> String {
    let s = c(s);
    let p = Path::new(&s);
    match File::open(p) {
        Ok(v) => match Decoder::new(BufReader::new(v)) {
            Ok(v) => {
                sink.clear();
                _ = sink.try_seek(Duration::ZERO);
                sink.append(v);
                sink.play();
                return p.file_name().unwrap().to_string_lossy().to_string();
            }
            _ => {}
        },
        _ => {}
    }
    t.to_string()
}
