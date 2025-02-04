# shotgun
_A very simple oneshot single producer, multiple consumer (SPMC) channel_

## About
Shotgun is a simple oneshot single producer, multiple consumer (SPMC) channel.
Internally using `std::sync::Mutex` and `std::sync::Arc`, not containing any unsafe code.
Currently limited to non-blocking functions.

## How to use

```rust
use shotgun::channel;
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
```

## License
[MIT](LICENSE)