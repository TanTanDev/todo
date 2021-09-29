use std::time::Instant;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use crossterm::event::{self, KeyCode, Event as CEvent};

//use termion::event::Key;
//use termion::input::TermRead;

pub enum Event<I> {
    Input(I),
    Tick,
}

/// A small event handler that wrap termion input and tick events. Each event
/// type is handled in its own thread and returned to a common `Receiver`
pub struct Events {
    rx: mpsc::Receiver<Event<KeyCode>>,
    _input_handle: thread::JoinHandle<()>,
}

#[derive(Debug, Clone, Copy)]
pub struct Config {
    pub tick_rate: Duration,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            tick_rate: Duration::from_millis(250),
        }
    }
}

impl Events {
    pub fn new() -> Events {
        Events::with_config(Config::default())
    }

    pub fn with_config(config: Config) -> Events {
        let (tx, rx) = mpsc::channel();
        let _input_handle = {
            let tx = tx.clone();
            thread::spawn(move || {
                //let stdin = io::stdin();
                let mut last_tick = Instant::now();
                loop {
                    let timeout = config.tick_rate
                        .checked_sub(last_tick.elapsed())
                        .unwrap_or_else(|| Duration::from_secs(0));
                    if event::poll(timeout).unwrap() {
                        if let CEvent::Key(key) = event::read().unwrap() {
                            tx.send(Event::Input(key.code)).unwrap();
                        }
                    }
                    if last_tick.elapsed() >= config.tick_rate {
                        tx.send(Event::Tick).unwrap();
                        last_tick = Instant::now();
                    }
                }
            }
        )};
        Events {
            rx,
            _input_handle,
        }
    }

    pub fn next(&self) -> Result<Event<KeyCode>, mpsc::RecvError> {
        self.rx.recv()
    }
}
