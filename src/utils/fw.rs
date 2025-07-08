use std::{
    fmt,
    fs::File,
    io::{self, Read, Seek},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use crossbeam::{
    channel::{Receiver, RecvError, SendError, Sender, unbounded},
    select,
};
use notify::{RecursiveMode, Watcher, event::ModifyKind};

/// Represents the file content and any metadata needed for display
#[derive(Debug, Clone)]
pub struct FileContent {
    pub content: String,
    pub is_truncated: bool,
}

/// Errors that can occur during file watching
pub enum FileWatcherError {
    Watcher(notify::Error),
    File(io::Error),
}

impl fmt::Display for FileWatcherError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            FileWatcherError::Watcher(e) => write!(f, "Watcher error: {}", e),
            FileWatcherError::File(e) => write!(f, "Read error: {}", e),
        }
    }
}

/// Message to control the file watcher
pub enum FileWatcherMessage {
    FilePath(Option<PathBuf>),
}

/// Public handle to control the file watcher
pub struct FileWatcherHandle {
    sender: Sender<FileWatcherMessage>,
    file_path: Option<PathBuf>,
}

impl FileWatcherHandle {
    pub fn new(
        content_sender: Sender<Result<FileContent, FileWatcherError>>,
        interval: Duration,
    ) -> Self {
        let (sender, receiver) = unbounded();
        let mut watcher = FileWatcher::new(content_sender, receiver, interval);

        thread::spawn(move || {
            if let Err(e) = watcher.run() {
                eprintln!("File watcher error: {:?}", e);
            }
        });

        Self {
            sender,
            file_path: None,
        }
    }

    pub fn set_file_path(&mut self, file_path: Option<PathBuf>) {
        if self.file_path != file_path {
            self.file_path = file_path.clone();
            if let Err(e) = self.sender.send(FileWatcherMessage::FilePath(file_path)) {
                eprintln!("Failed to send file path to watcher: {}", e);
            }
        }
    }
}

/// The actual file watcher implementation that runs in a background thread
struct FileWatcher {
    content_sender: Sender<Result<FileContent, FileWatcherError>>,
    receiver: Receiver<FileWatcherMessage>,
    file_path: Option<PathBuf>,
    interval: Duration,
}

impl FileWatcher {
    fn new(
        content_sender: Sender<Result<FileContent, FileWatcherError>>,
        receiver: Receiver<FileWatcherMessage>,
        interval: Duration,
    ) -> Self {
        FileWatcher {
            content_sender,
            receiver,
            file_path: None,
            interval,
        }
    }

    fn run(&mut self) -> Result<(), RecvError> {
        eprintln!("Starting file watcher");
        // Setup the file watcher channel
        let (watch_sender, watch_receiver) = unbounded();

        // Create a watcher instance that will send path events when files change
        let mut watcher =
            match notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
                if let Ok(event) = res {
                    match event.kind {
                        notify::EventKind::Modify(ModifyKind::Data(_)) => {
                            // Send file paths that have been modified
                            if let Err(e) = watch_sender.send(event.paths) {
                                eprintln!("Error sending watch event: {}", e);
                            }
                        }
                        _ => {}
                    };
                }
            }) {
                Ok(watcher) => watcher,
                Err(e) => {
                    let _ = self.content_sender.send(Err(FileWatcherError::Watcher(e)));
                    return Ok(());
                }
            };

        // Create channels for the FileReader
        let (mut content_sender, mut content_receiver) = unbounded::<io::Result<FileContent>>();
        let (mut notify_sender, mut notify_receiver) = unbounded::<()>();

        // Create a shared file reader state
        let reader_state = Arc::new(Mutex::new(None::<FileReader>));

        loop {
            select! {
                // Handle messages to the file watcher
                recv(self.receiver) -> msg => {
                    match msg? {
                        FileWatcherMessage::FilePath(file_path) => {
                            // Reset channels
                            (content_sender, content_receiver) = unbounded();
                            (notify_sender, notify_receiver) = unbounded::<()>();

                            // Stop watching the previous path
                            if let Some(p) = &self.file_path {
                                if let Err(e) = watcher.unwatch(p) {
                                    let _ = self.content_sender.send(Err(FileWatcherError::Watcher(e)));
                                }
                                self.file_path = None;

                                // Stop the previous reader
                                let mut state = reader_state.lock().unwrap();
                                *state = None;
                            }

                            // Start watching the new path
                            if let Some(p) = file_path {
                                let res = watcher.watch(Path::new(&p), RecursiveMode::NonRecursive);
                                match res {
                                    Ok(_) => {
                                        self.file_path = Some(p.clone());

                                        // Create a new file reader
                                        let reader = FileReader::new(
                                            content_sender.clone(),
                                            notify_receiver.clone(),
                                            p.clone(),
                                            self.interval,
                                        );

                                        // Store the reader state
                                        let reader_clone = Arc::clone(&reader_state);
                                        *reader_state.lock().unwrap() = Some(reader);

                                        // Spawn a thread to run the reader
                                        thread::spawn(move || {
                                            if let Some(mut reader) = reader_clone.lock().unwrap().take() {
                                                if let Err(e) = reader.run() {
                                                    eprintln!("File reader error: {:?}", e);
                                                }
                                            }
                                        });

                                        // Do an initial read of the file
                                        eprintln!("Initial read request for path: {:?}", p);
                                        eprintln!("Starting from end of file");
                                        let _ = notify_sender.send(());
                                    },
                                    Err(e) => {
                                        let _ = self.content_sender.send(Err(FileWatcherError::Watcher(e)));
                                    }
                                };
                            } else {
                                // No file to watch, send empty content
                                eprintln!("No file to watch, sending empty content");
                                let content = FileContent {
                                    content: String::new(),
                                    is_truncated: false,
                                };
                                let _ = content_sender.send(Ok(content));
                            }
                        }
                    }
                }

                // Handle file change notifications from notify
                recv(watch_receiver) -> msg => {
                    if let Ok(_) = msg {
                        // Notify the file reader to check for changes
                        let _ = notify_sender.send(());
                    }
                }

                // Handle content updates from the file reader
                recv(content_receiver) -> msg => {
                    if let Ok(content_result) = msg {
                        // Forward the content to the app
                        let result = content_result.map_err(FileWatcherError::File);
                        eprintln!("Forwarding content to app: {} bytes",
                                 result.as_ref().map(|c| c.content.len()).unwrap_or(0));
                        if let Err(e) = self.content_sender.send(result) {
                            eprintln!("Failed to send content to app: {}", e);
                        }
                    }
                }
            }
        }
    }
}

/// Reader that reads file content and tracks position
struct FileReader {
    content_sender: Sender<io::Result<FileContent>>,
    receiver: Receiver<()>,
    file_path: PathBuf,
    interval: Duration,
    content: String,
    pos: u64,
}

impl FileReader {
    fn new(
        content_sender: Sender<io::Result<FileContent>>,
        receiver: Receiver<()>,
        file_path: PathBuf,
        interval: Duration,
    ) -> Self {
        // Get the file size to start reading from the end
        let pos = match File::open(&file_path) {
            Ok(file) => match file.metadata() {
                Ok(metadata) => metadata.len(),
                Err(_) => 0,
            },
            Err(_) => 0,
        };

        FileReader {
            content_sender,
            receiver,
            file_path,
            interval,
            content: String::new(),
            pos,
        }
    }

    fn run(&mut self) -> Result<(), ()> {
        // Do an initial read of the file (will send current state)
        self.update().map_err(|_| ())?;

        loop {
            select! {
                // Handle file change notifications
                recv(self.receiver) -> msg => {
                    msg.map_err(|_| ())?;
                    self.update().map_err(|_| ())?;
                }

                // Fallback polling in case the file watcher doesn't work
                // (e.g. on network filesystems)
                default(self.interval) => {
                    self.update().map_err(|_| ())?;
                }
            }
        }
    }

    fn update(&mut self) -> Result<(), SendError<io::Result<FileContent>>> {
        // Clear the content buffer for this update
        self.content.clear();

        let result = File::open(&self.file_path).and_then(|mut f| {
            // Check if file was truncated (size is smaller than our position)
            let metadata = f.metadata()?;
            let is_truncated = metadata.len() < self.pos;

            // If file was truncated, start from the beginning
            if is_truncated {
                self.pos = 0;
            }

            // Seek to the last read position
            self.pos = f.seek(io::SeekFrom::Start(self.pos))?;

            // Read new content
            let mut raw_content = String::new();
            let bytes_read = f.read_to_string(&mut raw_content)? as u64;
            self.pos += bytes_read;

            // If this is the first read and we're at end of file (no content read),
            // send an empty string but don't change position
            if raw_content.is_empty() && self.content.is_empty() {
                eprintln!("At end of file, no new content to read");
            }

            // Process raw content to handle \r
            let processed_content = raw_content
                .split('\n')
                .map(|line| line.replace('\r', ""))
                .collect::<Vec<_>>()
                .join("\n");

            self.content = processed_content;
            // Log when we're reading content
            if !self.content.is_empty() {
                eprintln!(
                    "Read {} bytes of new content from {}",
                    bytes_read,
                    self.file_path.display()
                );
            }

            Ok(FileContent {
                content: self.content.clone(),
                is_truncated,
            })
        });

        self.content_sender.send(result)
    }
}
