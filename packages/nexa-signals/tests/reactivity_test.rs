use nexa_signals::{create_effect, create_memo, dependency::batch, signal};
use std::cell::RefCell;
use std::rc::Rc;

#[test]
fn test_basic_update() {
    let count = signal(0);
    let doubled = create_memo({
        let count = count.clone();
        move || count.get() * 2
    });

    assert_eq!(doubled.get(), 0);

    count.set(10);
    // Auto-propagation happens if no batch

    assert_eq!(doubled.get(), 20);
}

#[test]
fn test_diamond_problem() {
    // A -> B
    // A -> C
    // B + C -> D
    // Update A, D should update once.

    let a = signal(1);
    let b = create_memo({
        let a = a.clone();
        move || a.get() * 2
    });
    let c = create_memo({
        let a = a.clone();
        move || a.get() + 1
    });

    let executions = Rc::new(RefCell::new(0));
    let d = create_memo({
        let b = b.clone();
        let c = c.clone();
        let exec = executions.clone();
        move || {
            *exec.borrow_mut() += 1;
            b.get() + c.get()
        }
    });

    // Initial run
    assert_eq!(d.get(), 1 * 2 + (1 + 1)); // 2 + 2 = 4
    assert_eq!(*executions.borrow(), 1);

    // Update A
    a.set(2);
    // A=2 -> B=4, C=3 -> D=7

    assert_eq!(d.get(), 7);
    assert_eq!(*executions.borrow(), 2, "Should execute exactly once more");
}

#[test]
fn test_batching() {
    let a = signal(0);
    let executions = Rc::new(RefCell::new(0));

    let _effect = create_effect({
        let a = a.clone();
        let exec = executions.clone();
        move || {
            a.get(); // read
            *exec.borrow_mut() += 1;
        }
    });

    assert_eq!(*executions.borrow(), 1); // Initial run

    batch(|| {
        a.set(1);
        a.set(2);
        a.set(3);
    });

    assert_eq!(*executions.borrow(), 2);
    assert_eq!(a.get(), 3);
}

#[test]
#[should_panic(expected = "Cyclic dependency detected")]
fn test_cycle_detection() {
    use std::cell::RefCell;
    let b_ref: Rc<RefCell<Option<nexa_signals::Memo<i32>>>> = Rc::new(RefCell::new(None));
    let s = signal(0);

    let s_clone = s.clone();
    let b_ref_clone = b_ref.clone();

    let a = create_memo(move || {
        s_clone.get();
        if let Some(b) = &*b_ref_clone.borrow() {
            b.get()
        } else {
            0
        }
    });

    let b = create_memo({
        let a = a.clone();
        move || a.get() + 1
    });

    *b_ref.borrow_mut() = Some(b);

    // Trigger update on S to re-run A.
    // A will read B. B depends on A.
    // Cycle A -> B -> A.
    s.set(1);
}

#[test]
fn test_effect_cleanup() {
    let s = signal(0);
    let executions = Rc::new(RefCell::new(0));

    {
        let _e = create_effect({
            let s = s.clone();
            let exec = executions.clone();
            move || {
                s.get();
                *exec.borrow_mut() += 1;
            }
        });

        assert_eq!(*executions.borrow(), 1);
        s.set(1);
        assert_eq!(*executions.borrow(), 2);
    } // _e dropped here

    s.set(2);
    assert_eq!(*executions.borrow(), 2, "Effect should not run after drop");
}

#[test]
fn test_node_cleanup() {
    // Verify that dropping a signal removes it from the graph
    // and listeners stop receiving updates.

    let executions = Rc::new(RefCell::new(0));

    {
        let s = signal(10);
        let _memo = create_memo({
            let s = s.clone();
            let exec = executions.clone();
            move || {
                *exec.borrow_mut() += 1;
                s.get() * 2
            }
        });

        assert_eq!(*executions.borrow(), 1);
        s.set(20);
        assert_eq!(*executions.borrow(), 2);

        // s goes out of scope here
    }

    // Test dropping the MEMO while keeping signal.

    let s = signal(10);
    {
        let _memo = create_memo({
            let s = s.clone();
            let exec = executions.clone();
            move || {
                *exec.borrow_mut() += 1;
                s.get()
            }
        });
        // executions is 2 from prev block + 1 initial here = 3
        assert_eq!(*executions.borrow(), 3);

        s.set(20);
        assert_eq!(*executions.borrow(), 4);
    } // _memo dropped

    s.set(30);
    assert_eq!(
        *executions.borrow(),
        4,
        "Memo should stop updating after drop"
    );
}

#[test]
fn test_deep_dependency_chain() {
    // A -> B -> C ... -> Z (50 deep)

    let root = signal(0);
    let mut current = create_memo({
        let root = root.clone();
        move || root.get() + 1
    });

    for _ in 0..50 {
        let prev = current.clone();
        current = create_memo(move || prev.get() + 1);
    }

    // root=0
    // initial 'current' is 1. (Depth 1)
    // LOOP 0: new 'current' is prev(1) + 1 = 2.
    // ...
    // LOOP 49: new 'current' is prev(50) + 1 = 51.
    // So expected is 51.

    assert_eq!(current.get(), 51);

    root.set(10);
    // Should propagate all the way
    // 10 + 1 + 50 = 61.
    assert_eq!(current.get(), 61);
}

#[test]
fn test_fan_out_updates() {
    // One signal -> 100 memos
    // Verify all update

    let root = signal(0);
    let mut memos = Vec::new();
    let executions = Rc::new(RefCell::new(0));

    for _ in 0..100 {
        let root = root.clone();
        let exec = executions.clone();
        memos.push(create_memo(move || {
            *exec.borrow_mut() += 1;
            root.get()
        }));
    }

    assert_eq!(*executions.borrow(), 100);

    root.set(1);

    assert_eq!(*executions.borrow(), 200);

    for memo in memos {
        assert_eq!(memo.get(), 1);
    }
}
