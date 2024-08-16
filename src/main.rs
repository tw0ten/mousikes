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
    fs::File,
    io::{self, stdout, BufReader},
    time::Duration,
};

fn complete(s: &String) -> String {
    if s == "" {
        return String::from("gk");
    }
    String::from("")
}

const IDLE_POLL: Duration = Duration::from_millis(5000);

fn main() -> io::Result<()> {
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    let (_stream, stream_handle) = OutputStream::try_default().unwrap();
    let sink = Sink::try_new(&stream_handle).unwrap();
    sink.set_volume(0.5);

    let source = Decoder::new(BufReader::new(File::open("i/gk.mp3").unwrap())).unwrap();
    sink.append(source);
    let _ = sink.try_seek(Duration::from_millis(500));

    let mut playlist = String::from("/");
    let mut text = String::new();
    let seek = Duration::from_millis(1500);

    let mut interval = IDLE_POLL;
    loop {
        terminal.draw(|frame: &mut Frame| {
            frame.render_widget(
                Paragraph::new(format!(
                    "{{{}{}}}\n{} <{}>\n[{}] ({})",
                    text,
                    complete(&text),
                    if sink.is_paused() { "=" } else { "+" },
                    sink.volume(),
                    playlist,
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
                        KeyCode::Left => match sink.try_seek(if sink.get_pos() > seek {
                            sink.get_pos() - seek
                        } else {
                            Duration::from_millis(0)
                        }) {
                            Ok(_) => {}
                            _ => {}
                        },
                        KeyCode::Right => match sink.try_seek(sink.get_pos() + seek) {
                            Ok(_) => {}
                            _ => {}
                        },
                        KeyCode::Enter => {
                            if text != "" {
                                text.push_str(&complete(&text));
                                playlist = text;
                                text = String::new();
                            }
                        }
                        KeyCode::Backspace => text = String::new(),
                        KeyCode::Char(v) => text.push_str(&String::from(v)),
                        _ => {}
                    },
                    _ => {}
                },
                Event::Paste(s) => text.push_str(&s),
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
