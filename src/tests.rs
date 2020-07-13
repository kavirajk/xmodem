use crate::*;
use std::io::Cursor;

#[test]
fn test_read_byte() {
    let mut x = Xmodem::new(Cursor::new(vec![0, 2, 3]));

    assert_eq!(x.read_byte().expect("expect 0"), 0);
    assert_eq!(x.read_byte().expect("expect 2"), 2);
    assert_eq!(x.read_byte().expect("expect 3"), 3);
}

#[test]
fn test_write_byte() {
    let mut v = vec![0, 0, 0];

    let mut x = Xmodem::new(Cursor::new(&mut v));

    x.write_byte(1);
    x.write_byte(2);
    x.write_byte(3);

    assert_eq!(v, vec![1, 2, 3]);
}

#[test]
fn test_expect_byte_or_cancel() {
    let mut v = vec![2, 0];

    let mut x = Xmodem::new(Cursor::new(&mut v));

    x.expect_byte_or_cancel(1, "expected 1")
        .expect_err("error expected");

    assert_eq!(v[1], CAN);

    v = vec![2, 0];

    x = Xmodem::new(Cursor::new(&mut v));

    x.expect_byte_or_cancel(2, "expected 2").ok();

    assert_eq!(v[1], 0);
}

#[test]
fn test_small_packet_eof_error() {
    let mut xmodem = Xmodem::new(Cursor::new(vec![NAK, NAK, NAK]));

    let mut buffer = [1, 2, 3];
    let e = xmodem.read_packet(&mut buffer[..]).expect_err("read EOF");
    assert_eq!(e.kind(), io::ErrorKind::UnexpectedEof);

    // let e = xmodem.write_packet(&buffer).expect_err("write EOF");
    // assert_eq!(e.kind(), io::ErrorKind::UnexpectedEof);
}
