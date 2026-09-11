// Wraps the async file dialog so it can be polled from egui's synchronous
// update loop without blocking the UI thread while Android's system
// file picker is open.
use std::sync::{Arc, Mutex};

pub struct FilePicker {
    // Holds the picked file's raw bytes once the background thread finishes.
    // None while idle or while a pick is still in progress.
    result: Arc<Mutex<Option<Vec<u8>>>>,
    picking: bool,
}

impl FilePicker {
    pub fn new() -> Self {
        Self {
            result: Arc::new(Mutex::new(None)),
            picking: false,
        }
    }

    // Call this from a button's on-click. Spawns a background thread that
    // opens the system file picker and blocks on it there, not on the UI thread.
    pub fn open(&mut self) {
        if self.picking {
            return; // a pick is already in progress, ignore repeated clicks
        }
        self.picking = true;
        let slot = Arc::clone(&self.result);

        std::thread::spawn(move || {
            let picked = pollster::block_on(async {
                rfd::AsyncFileDialog::new()
                    .add_filter("Liphia script", &["lph"])
                    .set_title("Open Liphia script")
                    .pick_file()
                    .await
            });

            if let Some(handle) = picked {
                let bytes = pollster::block_on(handle.read());
                *slot.lock().unwrap() = Some(bytes);
            }
        });
    }

    // Call this once per frame from App::ui. Returns Some(source_code) exactly
    // once, the frame after the background thread finishes reading the file.
    pub fn poll(&mut self) -> Option<String> {
        let mut guard = self.result.lock().unwrap();
        if let Some(bytes) = guard.take() {
            self.picking = false;
            return String::from_utf8(bytes).ok();
        }
        None
    }

    pub fn is_picking(&self) -> bool {
        self.picking
    }
}