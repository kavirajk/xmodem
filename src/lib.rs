mod progress;

#[cfg(test)]
mod tests;

use progress::*;
use std::io::{self, Read, Result, Write};

pub const SOH: u8 = 0x01;
pub const EOT: u8 = 0x04;
pub const ACK: u8 = 0x06;
pub const NAK: u8 = 0x15;
pub const CAN: u8 = 0x18;

struct Xmodem<T> {
    inner: T,
    packet: u8,
    started: bool,
    progress: ProgressFn,
}

impl Xmodem<()> {
    pub fn transmit<R, W>(from: R, to: W) -> io::Result<usize>
    where
        W: Read + Write,
        R: Read,
    {
        Self::transmit_with_progress(from, to, progress::noop)
    }

    pub fn transmit_with_progress<R, W>(
        mut from: R,
        to: W,
        progress: ProgressFn,
    ) -> io::Result<usize>
    where
        W: Read + Write,
        R: Read,
    {
        let mut packet = [0u8; 128];
        let mut written = 0;

        let mut transmitter = Xmodem::new_with_progress(to, progress);

        // till no more data is available
        'next_packet: loop {
            // read packet from srouce.
            let n = from.read(&mut packet)?;

            // use xmode to transform single packet

            if n == 0 {
                // no more data
                transmitter.write_packet(&[])?;
                return Ok(written);
            }

            for _ in 0..10 {
                match transmitter.write_packet(&packet) {
                    Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                    Ok(_) => {
                        written += n;
                        continue 'next_packet;
                    }
                    Err(e) => return Err(e),
                }
            }
            return Err(io::Error::new(io::ErrorKind::BrokenPipe, "bad transmit"));
        }
    }

    pub fn receive<R, W>(from: R, to: W) -> io::Result<usize>
    where
        W: Write,
        R: Read + Write,
    {
        Self::receive_with_progress(from, to, progress::noop)
    }

    pub fn receive_with_progress<R, W>(
        from: R,
        mut to: W,
        progress: ProgressFn,
    ) -> io::Result<usize>
    where
        W: Write,
        R: Read + Write,
    {
        let mut packet = [0u8; 128];
        let mut received = 0;

        let mut receiver = Xmodem::new_with_progress(from, progress);

        'next_packet: loop {
            for _ in 0..10 {
                match receiver.read_packet(&mut packet) {
                    Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                    Err(e) => return Err(e),
                    Ok(0) => break 'next_packet,
                    Ok(n) => {
                        received += n;
                        to.write_all(&packet)?;
                        continue 'next_packet;
                    }
                }
            }

            return Err(io::Error::new(io::ErrorKind::BrokenPipe, "bad receive"));
        }

        Ok(received)
    }
}

impl<T: Read + Write> Xmodem<T> {
    pub fn new(inner: T) -> Self {
        Self::new_with_progress(inner, progress::noop)
    }

    pub fn new_with_progress(inner: T, progress: ProgressFn) -> Self {
        Xmodem {
            inner,
            packet: 1,
            started: false,
            progress,
        }
    }

    fn read_byte(&mut self) -> Result<u8> {
        let mut buf = [0u8; 1];
        self.inner.read(&mut buf)?;
        Ok(buf[0])
    }

    fn write_byte(&mut self, byte: u8) -> Result<()> {
        self.inner.write(&mut [byte])?;
        Ok(())
    }

    // Read next byte from `inner` return `byte` if its same as `byte`.
    // If differs, send `CAN` to `inner`
    // If read byte is `CAN` send `Connectionaborted`
    // Else return `Invaliddata`
    fn expect_byte_or_cancel(&mut self, byte: u8, msg: &'static str) -> Result<u8> {
        let b = self.read_byte()?;
        if b == byte {
            return Ok(byte);
        }
        self.inner.write(&mut [CAN])?;

        match b {
            CAN => Err(io::Error::new(io::ErrorKind::ConnectionAborted, msg)),
            _ => {
                println!("got: {}", b);
                Err(io::Error::new(io::ErrorKind::InvalidData, msg))
            }
        }
    }

    // Same as `expect_byte_or_cancel` except, it shouldn't send `CAN` to `inner`
    // if next byte differs.
    fn expect_byte(&mut self, byte: u8, msg: &'static str) -> Result<u8> {
        let b = self.read_byte()?;
        if b == byte {
            return Ok(byte);
        }

        match b {
            CAN => Err(io::Error::new(io::ErrorKind::ConnectionAborted, msg)),
            _ => Err(io::Error::new(io::ErrorKind::InvalidData, msg)),
        }
    }

    pub fn read_packet(&mut self, buf: &mut [u8]) -> Result<usize> {
        if buf.len() < 128 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "unexpectedeof",
            ));
        }

        if !self.started {
            // send NAK to initiate the transmit.
            self.write_byte(NAK)?;
            (self.progress)(Progress::Started);
            self.started = true;
        }

        match self.read_byte()? {
            EOT => {
                // end of the transmission
                self.write_byte(NAK)?;
                self.expect_byte(EOT, "expected second EOT")?;
                self.write_byte(ACK)?;
                self.started = false;
                Ok(0)
            }
            SOH => {
                self.expect_byte_or_cancel(self.packet, "did not match current packet number")?;
                self.expect_byte_or_cancel(
                    !self.packet,
                    "did not match packet numbers's 1's complement",
                )?;
                let n = self.inner.read(&mut buf[..])?;
                if n < 128 {
                    return Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "unexpectedeof",
                    ));
                }
                let mut csum = 0u8;
                for i in 0..128 {
                    csum = csum.wrapping_add(buf[i])
                }

                let b = self.read_byte()?;

                if csum != b {
                    self.write_byte(NAK)?;
                    return Err(io::Error::new(io::ErrorKind::Interrupted, "bad checksum"));
                }

                self.write_byte(ACK)?;

                (self.progress)(Progress::Packet(self.packet));

                self.packet = self.packet.wrapping_add(1);

                Ok(128)
            }
            _ => {
                self.write_byte(CAN)?;
                Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "expected SOH or EOT",
                ))
            }
        }
    }

    pub fn write_packet(&mut self, buf: &[u8]) -> Result<usize> {
        if buf.len() != 0 && buf.len() < 128 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "unexpectedeof",
            ));
        }

        if !self.started {
            // wait for reciver to send NAK
            (self.progress)(Progress::Waiting);
            self.expect_byte(NAK, "expected NAK from receiver")?;
            (self.progress)(Progress::Started);
            self.started = true;
        }

        if buf.len() == 0 {
            // no more data. end the transmit
            self.write_byte(EOT)?;
            self.expect_byte(NAK, "expected NAK for EOT")?;
            self.write_byte(EOT)?;
            self.expect_byte(ACK, "expected 2nd NAK for EOT")?;
            self.started = false;
            return Ok(0);
        }

        // SOH to start begining of the packet
        self.write_byte(SOH)?;

        // packet number
        self.write_byte(self.packet)?;

        // 1's complement of packet number
        self.write_byte(!self.packet)?;

        // actual payload
        let n = self.inner.write(&buf[..])?;

        if n < 128 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "unexpectedeof",
            ));
        }

        let mut csum = 0u8;
        for i in 0..128 {
            csum = csum.wrapping_add(buf[i]);
        }

        // send checksum
        self.write_byte(csum)?;

        let b = self.read_byte()?;
        match b {
            NAK => Err(io::Error::new(io::ErrorKind::Interrupted, "expected")),
            ACK => {
                (self.progress)(Progress::Packet(self.packet));

                self.packet = self.packet.wrapping_add(1);

                self.flush()?;

                Ok(n)
            }
            _ => Err(io::Error::new(io::ErrorKind::InvalidData, "expected")),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}
