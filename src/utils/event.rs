use color_eyre::Result;
use crossterm::event::{
    self, Event as CrosstermEvent, KeyCode, KeyEvent, KeyModifiers, MouseEvent,
};
use std::{
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

/// Events that can be handled by the application
#[derive(Debug, Clone, Copy)]
pub enum Event {
    /// Terminal tick (for animations)
    Tick,
    /// Key press event
    Key(KeyEvent),
    /// Mouse click/scroll event
    Mouse(MouseEvent),
    /// Terminal resize event
    Resize(u16, u16),
}

/// Event handler configuration
#[derive(Debug, Clone, Copy)]
pub struct EventConfig {
    /// Duration between ticks
    pub tick_rate: Duration,
    /// Whether to capture mouse events
    pub enable_mouse_capture: bool,
}

impl Default for EventConfig {
    fn default() -> Self {
        Self {
            tick_rate: Duration::from_millis(250),
            enable_mouse_capture: true,
        }
    }
}

/// Event handler that listens for terminal events
pub struct EventHandler {
    /// Event receiver channel
    pub rx: mpsc::Receiver<Event>,
    /// Event sender channel
    tx: mpsc::Sender<Event>,
    /// Thread handle for the event handler
    handle: thread::JoinHandle<()>,
}

impl EventHandler {
    /// Create a new event handler with the given configuration
    pub fn new(config: EventConfig) -> Self {
        let (tx, rx) = mpsc::channel();
        let handle = {
            let tx = tx.clone();
            thread::spawn(move || {
                let tick_rate = config.tick_rate;
                let mut last_tick = Instant::now();

                loop {
                    let timeout = tick_rate
                        .checked_sub(last_tick.elapsed())
                        .unwrap_or(Duration::from_secs(0));

                    if event::poll(timeout).expect("Failed to poll for events") {
                        match event::read().expect("Failed to read event") {
                            CrosstermEvent::Key(key) => {
                                if tx.send(Event::Key(key)).is_err() {
                                    return;
                                }
                            }
                            CrosstermEvent::Mouse(mouse) => {
                                if tx.send(Event::Mouse(mouse)).is_err() {
                                    return;
                                }
                            }
                            CrosstermEvent::Resize(width, height) => {
                                if tx.send(Event::Resize(width, height)).is_err() {
                                    return;
                                }
                            }
                            _ => {}
                        }
                    }

                    if last_tick.elapsed() >= tick_rate {
                        if tx.send(Event::Tick).is_err() {
                            return;
                        }
                        last_tick = Instant::now();
                    }
                }
            })
        };

        Self { rx, tx, handle }
    }

    /// Checks if the given key event matches the provided code and modifiers
    pub fn is_key_with_modifiers(key: KeyEvent, code: KeyCode, modifiers: KeyModifiers) -> bool {
        key.code == code && key.modifiers == modifiers
    }

    /// Checks if the given key event matches any key with the provided modifiers
    pub fn is_any_key_with_modifiers(key: KeyEvent, modifiers: KeyModifiers) -> bool {
        key.modifiers == modifiers
    }

    /// Close the event handler
    pub fn close(&self) -> Result<()> {
        self.tx.send(Event::Key(KeyEvent::new(
            KeyCode::Char('q'),
            KeyModifiers::CONTROL,
        )))?;
        Ok(())
    }
}
