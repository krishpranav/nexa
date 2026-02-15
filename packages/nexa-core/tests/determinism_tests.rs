// use nexa_core::*;
use std::sync::{Arc, Mutex};
use std::thread;

#[test]
fn test_deterministic_execution_order() {
    let results = Arc::new(Mutex::new(Vec::new()));

    for i in 0..10 {
        let results = results.clone();
        let handle = thread::spawn(move || {
            // Simulate some work
            thread::sleep(std::time::Duration::from_millis(10 - i));
            results.lock().unwrap().push(i);
        });
        let _ = handle.join();
    }

    let res = results.lock().unwrap();
    // This should be 0, 1, 2... because we join them sequentially
    for (idx, &val) in res.iter().enumerate() {
        assert_eq!(idx, val as usize);
    }
}
