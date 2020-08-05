use metrics::{Identifier, Key, Recorder};
use std::sync::atomic::{AtomicUsize, Ordering};

#[allow(dead_code)]
static RECORDER: SimpleRecorder = SimpleRecorder::new();

struct SimpleRecorder {
    identifier_count: AtomicUsize,
}

impl SimpleRecorder {
    pub const fn new() -> SimpleRecorder {
        SimpleRecorder {
            identifier_count: AtomicUsize::new(0),
        }
    }
}

pub fn init_simple_recorder() {
    let recorder = SimpleRecorder::new();
    metrics::set_boxed_recorder(Box::new(recorder)).unwrap()
}

impl Recorder for SimpleRecorder {
    fn register_counter(&self, key: Key, _description: Option<&'static str>) -> Identifier {
        let id = self.identifier_count.fetch_add(1, Ordering::SeqCst);
        println!("(counter) mapping key {} to id {}", key, id);
        id.into()
    }

    fn register_gauge(&self, key: Key, _description: Option<&'static str>) -> Identifier {
        let id = self.identifier_count.fetch_add(1, Ordering::SeqCst);
        println!("(gauge) mapping key {} to id {}", key, id);
        id.into()
    }

    fn register_histogram(&self, key: Key, _description: Option<&'static str>) -> Identifier {
        let id = self.identifier_count.fetch_add(1, Ordering::SeqCst);
        println!("(histogram) mappi ng key {} to id {}", key, id);
        id.into()
    }

    fn increment_counter(&self, id: Identifier, value: u64) {
        println!("(counter) got value {} for id {:?}", value, id);
    }

    fn update_gauge(&self, id: Identifier, value: f64) {
        println!("(gauge) got value {} for id {:?}", value, id);
    }

    fn record_histogram(&self, id: Identifier, value: u64) {
        println!("(histogram) got value {} for id {:?}", value, id);
    }
}
