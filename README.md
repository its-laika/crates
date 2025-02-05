# Laika's namespace crate 

**This module contains multiple submodules (included via feature flags) with
different functionalities. They're all grouped under the `laika` namespace,
providing some kind of scoped crates (avoiding naming conflicts).**

## Submodules / Features

## shotgun
_A dead simple one-shot single producer, multiple consumer (SPMC) channel_

### About
Shotgun is a simple one-shot single producer, multiple consumer (SPMC) channel.
It internally uses `std::sync::Mutex` and `std::sync::Arc` and does not contain
any unsafe code.

### When to use

Likely when you need to pass a signal to multiple threads or functions to stop
in order to shut down the application.

### How to use

#### Synchronous
```rust
fn main() {
    use laika::shotgun::channel;
    use std::thread;
    use std::time;

    let (tx, rx) = channel();

    let rx1 = rx.clone();
    let thread1 = thread::spawn(move || loop {
        if rx1.try_recv().is_some() {
            return 1;
        }

        thread::sleep(time::Duration::from_secs(1));
    });

    let rx2 = rx.clone();
    let thread2 = thread::spawn(move || loop {
        if rx2.try_recv().is_some() {
            return 2;
        }

        thread::sleep(time::Duration::from_secs(1));
    });

    thread::sleep(time::Duration::from_secs(2));

    tx.send(()); // `tx` is dropped here.

    assert!(thread1.join().is_ok_and(|v| v == 1));
    assert!(thread2.join().is_ok_and(|v| v == 2));
}
```

#### Asynchronous
```rust
#[tokio]
async fn main() {
    use shotgun::channel;
    use std::thread;
    use std::time;

    let (tx, rx) = channel();

    let rx1 = rx.clone();
    let fun1 = async move {
        rx1.await;
        1
    };

    let rx2 = rx.clone();
    let fun2 = async move {
        // Explicit call to recv(), does the same as calling`.await` directly.
        rx2.recv().await;
        2
    };

    thread::sleep(time::Duration::from_secs(2));

    tx.send(());

    let rx3 = rx.clone();
    let fun3 = async move {
        rx3.await;
        3
    };

    let result = join!(fun1, fun2);

    assert_eq!(result.0, 1);
    assert_eq!(result.1, 2);
    assert_eq!(fun3.await, 3);
}
```

# License
[MIT](LICENSE)