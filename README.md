# Xmodem

A simple transfer protocol, the works surprisingly well and easy to implement.

You can find the good explanation of the protocol [here](https://cs140e.sergio.bz/assignments/1-shell/data/XMODEM.txt)

It is created as part of doing [cs140e](https://cs140e.sergio.bz/assignments/1-shell/#subphase-c-xmodem) assignments.

# Usage
```rust

use std::sync::mpsc::{channel, Receiver, Sender};

fn main() {
    let mut input = [0u8; 384];
    for (i, chunk) in input.chunks_mut(128).enumerate() {
        chunk.iter_mut().for_each(|b| *b = i as u8);
    }

    let (tx, rx) = pipe();
    let tx_thread = std::thread::spawn(move || Xmodem::transmit(&input[..], rx));
    let rx_thread = std::thread::spawn(move || {
        let mut output = [0u8; 384];
        Xmodem::receive(tx, &mut output[..]).map(|_| output)
    });

    assert_eq!(
        tx_thread.join().expect("tx join okay").expect("tx okay"),
        384
    );
    let output = rx_thread.join().expect("rx join okay").expect("rx okay");
    assert_eq!(&input[..], &output[..]);
}

struct Pipe(Sender<u8>, Receiver<u8>, Vec<u8>);

fn pipe() -> (Pipe, Pipe) {
    let ((tx1, rx1), (tx2, rx2)) = (channel(), channel());
    (Pipe(tx1, rx2, vec![]), Pipe(tx2, rx1, vec![]))
}

impl io::Read for Pipe {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        for i in 0..buf.len() {
            match self.1.recv() {
                Ok(byte) => buf[i] = byte,
                Err(_) => return Ok(i),
            }
        }

        Ok(buf.len())
    }
}

impl io::Write for Pipe {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        buf.iter().for_each(|b| self.2.push(*b));
        for (i, byte) in buf.iter().cloned().enumerate() {
            if let Err(e) = self.0.send(byte) {
                eprintln!("Write error: {}", e);
                return Ok(i);
            }
        }

        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
```

# LICENCE

MIT
