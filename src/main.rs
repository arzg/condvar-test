use crossterm::event;
use crossterm::terminal;
use parking_lot::Condvar;
use parking_lot::Mutex;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

fn main() -> crossterm::Result<()> {
    terminal::enable_raw_mode()?;
    crossterm::execute!(std::io::stdout(), event::EnableMouseCapture)?;

    let pair = Arc::new((Mutex::new("nothing"), Condvar::new()));
    let (keypress_tx, keypress_rx) = flume::unbounded();
    let (click_tx, click_rx) = flume::unbounded();
    let mut threads = Vec::new();

    threads.push(thread::spawn({
        let pair = Arc::clone(&pair);
        move || {
            let (m, c) = &*pair;
            printer(m, c)
        }
    }));
    threads.push(thread::spawn(|| event_listener(keypress_tx, click_tx)));
    threads.push(thread::spawn({
        let pair = Arc::clone(&pair);
        move || {
            let (m, c) = &*pair;
            keypress_listener(m, c, keypress_rx)
        }
    }));
    threads.push(thread::spawn({
        let pair = Arc::clone(&pair);
        move || {
            let (m, c) = &*pair;
            click_listener(m, c, click_rx)
        }
    }));

    for thread in threads {
        thread.join().unwrap();
    }

    terminal::disable_raw_mode()?;

    Ok(())
}

fn printer(m: &Mutex<&str>, c: &Condvar) {
    loop {
        let mut guard = m.lock();
        c.wait(&mut guard);
        println!("{}\r", guard);
    }
}

fn keypress_listener(m: &Mutex<&str>, c: &Condvar, keypress_rx: flume::Receiver<()>) {
    for () in keypress_rx {
        *m.lock() = "key pressed";
        c.notify_one();
    }
}

fn click_listener(m: &Mutex<&str>, c: &Condvar, click_rx: flume::Receiver<()>) {
    for () in click_rx {
        *m.lock() = "clicked";
        c.notify_one();
    }
}

fn event_listener(keypress_tx: flume::Sender<()>, click_tx: flume::Sender<()>) {
    loop {
        match event::read().unwrap() {
            event::Event::Key(event::KeyEvent {
                code: event::KeyCode::Char('c'),
                modifiers: event::KeyModifiers::CONTROL,
            }) => std::process::exit(0),

            event::Event::Key(_) => keypress_tx.send(()).unwrap(),

            event::Event::Mouse(event::MouseEvent {
                kind: event::MouseEventKind::Down(_),
                ..
            }) => click_tx.send(()).unwrap(),

            _ => {}
        }
    }
}
