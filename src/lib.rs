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
    pub fn transmit<R: Read>(r: R) -> Self {
        unimplemented!()
    }

    pub fn transmit_with_progress<R: Read>(r: R, progress: ProgressFn) -> Self {
        unimplemented!()
    }

    pub fn receive<W: Write>(w: W) -> Self {
        unimplemented!()
    }

    pub fn receive_with_progress<W: Write>(w: W, progress: ProgressFn) -> Self {
        unimplemented!()
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
            _ => Err(io::Error::new(io::ErrorKind::InvalidData, msg)),
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
                self.expect_byte_or_cancel(EOT, "expected second EOT")?;
                self.write_byte(ACK)?;
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

                self.packet.wrapping_add(1);

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

    pub fn write_packet(&mut self, buf: &mut [u8]) -> Result<usize> {
        unimplemented!()
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}
