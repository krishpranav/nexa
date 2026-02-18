use nexa_scheduler::{LocalScheduler, Scheduler};
use std::cell::RefCell;
use std::rc::Rc;

#[test]
fn test_execution_order() {
    let scheduler = LocalScheduler::new();
    let log = Rc::new(RefCell::new(Vec::new()));

    // Schedule tasks in mixed order
    {
        let log = log.clone();
        scheduler.schedule_effect(Box::new(move || {
            log.borrow_mut().push("effect");
        }));
    }

    {
        let log = log.clone();
        scheduler.schedule_layout_effect(Box::new(move || {
            log.borrow_mut().push("layout");
        }));
    }

    {
        let log = log.clone();
        scheduler.schedule_microtask(Box::new(move || {
            log.borrow_mut().push("microtask");
        }));
    }

    // Tick the scheduler
    scheduler.tick();

    // Verify order: Microtask -> Effect -> Layout
    let expected = vec!["microtask", "effect", "layout"];
    assert_eq!(*log.borrow(), expected);
}

#[test]
fn test_microtask_chaining() {
    // Microtasks scheduled by microtasks should run in the same tick (drain loop)
    let log = Rc::new(RefCell::new(Vec::new()));

    // We can't capture `&scheduler` in a `Box<dyn FnOnce()>` if the closure must be 'static?
    // Wait, the trait definition `Box<dyn FnOnce()>` implies 'static lifetime by default!
    // Ah, `Box<dyn FnOnce()>` is `Box<dyn FnOnce() + 'static>`.
    // So we can't capture local variables by reference. We need Rc/Arc/Weak or 'static.
    // BUT `LocalScheduler` is likely stack allocated in test.
    // Solution: Wrap scheduler in Rc for the test.

    let scheduler = Rc::new(LocalScheduler::new());

    {
        let log = log.clone();
        let sch = scheduler.clone();
        scheduler.schedule_microtask(Box::new(move || {
            log.borrow_mut().push("task1");

            // Schedule another
            let log = log.clone();
            sch.schedule_microtask(Box::new(move || {
                log.borrow_mut().push("task2");
            }));
        }));
    }

    scheduler.tick();

    assert_eq!(*log.borrow(), vec!["task1", "task2"]);
}
