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
    time::Duration,
};

const IDLE_POLL: Duration = Duration::from_millis(5000);

fn c(s: &String, p: &String, t: &String) -> String {
    if s == "" {
        return t.to_string();
    }
    for entry in fs::read_dir(p).unwrap() {
        let p = match entry {
            Ok(p) => match p.path().file_name() {
                Some(p) => p.to_string_lossy().to_string(),
                _ => String::new(),
            },
            _ => String::new(),
        };
        if p.starts_with(s) {
            return p.to_string();
        }
    }
    s.to_string()
}

fn e(s: &String, p: &String, sink: &Sink) -> (String, String) {
    let s = c(s,p,&String::new());
    let mut t = "".to_string();
    let mut p = p.to_string();
    let dir = s.ends_with("/");
    let s: Vec<&str> = s.split("/").collect();
    if s.len() > 1 || dir {
        p = s[0..s.len() - 1].join("/");
    }
    sink.clear();
    if !dir {
        t = s[s.len() - 1].to_string();
        let source = Decoder::new(BufReader::new(
            File::open(format!("./{}/{}", &p, &t)).unwrap(),
        ))
        .unwrap();
        sink.append(source);
        let _ = sink.try_seek(Duration::from_millis(0));
    }
    sink.play();
    (t, p)
}

fn main() -> io::Result<()> {
    let (_stream, stream_handle) = OutputStream::try_default().unwrap();
    let sink = Sink::try_new(&stream_handle).unwrap();
    sink.set_volume(0.5);
    let mut rng = rand::thread_rng();
    let mut s = String::new();
    {
        let args: Vec<String> = env::args().collect();
        if args.len() == 2 {
            s = args[1].to_string();
        }
    }
    let (mut t, mut p) = e(&s, &String::from("."), &sink);
    s = String::new();

    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    let seek = Duration::from_millis(1500);
    let mut interval = IDLE_POLL;
    loop {
        if sink.empty() {
            let files: Vec<_> = fs::read_dir(&p)
                .unwrap()
                .filter(|f| !f.as_ref().unwrap().path().is_dir())
                .collect();
            (t, p) = e(
                &format!(
                    "{}",
                    files[rng.gen_range(0..files.len())]
                        .as_ref()
                        .unwrap()
                        .file_name()
                        .to_string_lossy()
                ),
                &p,
                &sink,
            );
        }

        terminal.draw(|frame: &mut Frame| {
            frame.render_widget(
                Paragraph::new(format!(
                    "{{{}}}\n{} <{}>\n[{}] ({})",
                    c(&s, &p, &t),
                    if sink.is_paused() { "=" } else { "+" },
                    sink.volume(),
                    &p,
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
                        KeyCode::Tab => match sink.is_paused() {
                            true => sink.play(),
                            false => sink.pause(),
                        },
                        KeyCode::Up => sink.set_volume(
                            (1000.min((sink.volume() * 1000.0) as i32 + 25) as f32) / 1000.0,
                        ),
                        KeyCode::Down => sink.set_volume(
                            (0.max((sink.volume() * 1000.0) as i32 - 25) as f32) / 1000.0,
                        ),
                        KeyCode::Left => {
                            let _ = sink.try_seek(if sink.get_pos() > seek {
                                sink.get_pos() - seek
                            } else {
                                Duration::from_millis(0)
                            });
                        }
                        KeyCode::Right => {
                            let _ = sink.try_seek(sink.get_pos() + seek * 10);
                            sink.play();
                        }
                        KeyCode::Enter => {
                            (t, p) = e(&s, &p, &sink);
                            s = String::new();
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
