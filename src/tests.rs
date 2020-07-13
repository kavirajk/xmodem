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

    x.write_byte(1).expect("expect 1");
    x.write_byte(2).expect("expect 2");
    x.write_byte(3).expect("expect 3");

    assert_eq!(v, vec![1, 2, 3]);
}

// #[test]
// fn read_byte() {
//     let byte = Xmodem::new(Cursor::new(vec![CAN]))
//         .read_byte(false)
//         .expect("read a byte");

//     assert_eq!(byte, CAN);

//     let e = Xmodem::new(Cursor::new(vec![CAN]))
//         .read_byte(true)
//         .expect_err("abort on CAN");

//     assert_eq!(e.kind(), io::ErrorKind::ConnectionAborted);
// }

#[test]
fn test_expect_byte() {
    let mut xmodem = Xmodem::new(Cursor::new(vec![1, 1]));
    assert_eq!(xmodem.expect_byte(1, "1").expect("expected"), 1);
    let e = xmodem
        .expect_byte(2, "1, please")
        .expect_err("expect the unexpected");
    assert_eq!(e.kind(), io::ErrorKind::InvalidData);
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
fn test_expect_can() {
    let mut xmodem = Xmodem::new(Cursor::new(vec![CAN]));
    assert_eq!(xmodem.expect_byte(CAN, "hi").expect("CAN"), CAN);
}

#[test]
fn test_unexpected_can() {
    let e = Xmodem::new(Cursor::new(vec![CAN]))
        .expect_byte(SOH, "want SOH")
        .expect_err("have CAN");

    assert_eq!(e.kind(), io::ErrorKind::ConnectionAborted);
}

#[test]
fn test_cancel_on_unexpected() {
    let mut buffer = vec![CAN, 0];
    let e = Xmodem::new(Cursor::new(buffer.as_mut_slice()))
        .expect_byte_or_cancel(SOH, "want SOH")
        .expect_err("have CAN");

    assert_eq!(e.kind(), io::ErrorKind::ConnectionAborted);
    assert_eq!(buffer[1], CAN);

    let mut buffer = vec![0, 0];
    let e = Xmodem::new(Cursor::new(buffer.as_mut_slice()))
        .expect_byte_or_cancel(SOH, "want SOH")
        .expect_err("have 0");

    assert_eq!(e.kind(), io::ErrorKind::InvalidData);
    assert_eq!(buffer[1], CAN);
}

#[test]
fn test_small_packet_eof_error() {
    let mut xmodem = Xmodem::new(Cursor::new(vec![NAK, NAK, NAK]));

    let mut buffer = [1, 2, 3];
    let e = xmodem.read_packet(&mut buffer[..]).expect_err("read EOF");
    assert_eq!(e.kind(), io::ErrorKind::UnexpectedEof);

    let e = xmodem.write_packet(&buffer).expect_err("write EOF");
    assert_eq!(e.kind(), io::ErrorKind::UnexpectedEof);
}

#[test]
fn test_eot() {
    let mut buffer = vec![NAK, 0, NAK, 0, ACK];
    Xmodem::new(Cursor::new(buffer.as_mut_slice()))
        .write_packet(&[])
        .expect("write empty buf for EOT");

    assert_eq!(&buffer[..], &[NAK, EOT, NAK, EOT, ACK]);
}
