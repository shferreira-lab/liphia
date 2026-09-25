// Wraps the async file dialog so it can be polled from egui's synchronous
// update loop without blocking the UI thread while the system file picker
// is open. Uses rfd's async dialog on a background thread.
use std::sync::{Arc, Mutex};

pub struct FilePicker {
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

    pub fn open(&mut self) {
        if self.picking {
            return;
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

impl Default for FilePicker {
    fn default() -> Self {
        Self::new()
    }
}
