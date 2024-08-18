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

const IDLE_POLL: Duration = Duration::from_millis(5000);
const SEEK: Duration = Duration::from_millis(1500);

fn main() -> io::Result<()> {
    let (_stream, stream_handle) = OutputStream::try_default().unwrap();
    let sink = Sink::try_new(&stream_handle).unwrap();
    sink.set_volume(0.25);
    let mut rng = rand::thread_rng();
    let mut s = String::new();
    let mut t = String::new();

    let mut interval = IDLE_POLL;
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    let location = &match env::current_dir() {
        Ok(v) => v.display().to_string(),
        _ => String::from("."),
    };

    loop {
        if sink.empty() {
            let files: Vec<_> = fs::read_dir(".")
                .unwrap()
                .filter(|f| !f.as_ref().unwrap().path().is_dir())
                .collect();
            if files.len() > 0 {
                t = e(
                    &files[rng.gen_range(0..files.len())]
                        .as_ref()
                        .unwrap()
                        .file_name()
                        .to_string_lossy()
                        .to_string(),
                    &t,
                    &sink,
                );
            }
        }

        terminal.draw(|frame: &mut Frame| {
            frame.render_widget(
                Paragraph::new(format!(
                    "{{{}}}\n{} <{}>\n[{}] ({})",
                    if s == "" { t.to_string() } else { c(&s) },
                    if sink.is_paused() { "=" } else { "+" },
                    sink.volume(),
                    location,
                    sink.get_pos().as_secs()
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
                            (1000.min((sink.volume() * 1000.0) as i32 + 25) as f32) / 1000.0,
                        ),
                        KeyCode::Down => sink.set_volume(
                            (0.max((sink.volume() * 1000.0) as i32 - 25) as f32) / 1000.0,
                        ),
                        KeyCode::Left => {
                            let _ = sink.try_seek(if sink.get_pos() > SEEK {
                                sink.get_pos() - SEEK
                            } else {
                                Duration::from_millis(0)
                            });
                        }
                        KeyCode::Right => {
                            let _ = sink.try_seek(sink.get_pos() + SEEK * 10);
                            sink.play();
                        }
                        KeyCode::Enter => {
                            if s == "" {
                                match sink.is_paused() {
                                    true => sink.play(),
                                    false => sink.pause(),
                                };
                            } else {
                                t = e(&s, &t, &sink);
                                s = String::new();
                            }
                        }
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

fn c(s: &String) -> String {
    for p in fs::read_dir(".").unwrap() {
        let p = p.expect("").path();
        if p.is_dir() {
            continue;
        }
        let p = p.file_name().unwrap().to_string_lossy().to_string();
        if p.starts_with(s) {
            return p.to_string();
        }
    }
    s.to_string()
}

fn e(s: &String, t: &String, sink: &Sink) -> String {
    let s = c(s);
    let p = Path::new(&s);
    if p.exists() && !p.is_dir() {
        match Decoder::new(BufReader::new(File::open(p).unwrap())) {
            Ok(v) => {
                sink.clear();
                sink.append(v);
                let _ = sink.try_seek(Duration::from_millis(0));
                sink.play();
                return p.file_name().unwrap().to_string_lossy().to_string();
            }
            _ => {}
        }
    }
    t.to_string()
}
