//! Laws for bounded stdio message admission.

use std::future::Future;

use super::*;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

fn run_async(test: impl Future<Output = ()>) {
    tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap()
        .block_on(test);
}

#[test]
fn independent_lines_each_receive_the_complete_bound() {
    run_async(async {
        let (mut source, sink) = tokio::io::duplex(64);
        source.write_all(b"1234\n5678\n").await.unwrap();
        drop(source);

        let mut bounded = BoundedLines::new(sink, 4);
        let mut bytes = Vec::new();
        bounded.read_to_end(&mut bytes).await.unwrap();
        assert_eq!(bytes, b"1234\n5678\n");
    });
}

#[test]
fn oversized_line_fails_before_forwarding_the_excess_byte() {
    run_async(async {
        let (mut source, sink) = tokio::io::duplex(64);
        source.write_all(b"12345\n").await.unwrap();
        drop(source);

        let mut bounded = BoundedLines::new(sink, 4);
        let mut bytes = Vec::new();
        let error = bounded.read_to_end(&mut bytes).await.unwrap_err();
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
        assert_eq!(bytes, b"1234");
    });
}

#[test]
fn rejection_is_terminal_for_the_transport() {
    run_async(async {
        let (mut source, sink) = tokio::io::duplex(64);
        source.write_all(b"12345\nnext\n").await.unwrap();
        drop(source);

        let mut bounded = BoundedLines::new(sink, 4);
        let mut bytes = Vec::new();
        bounded.read_to_end(&mut bytes).await.unwrap_err();
        let mut retry = [0_u8; 1];
        let error = bounded.read(&mut retry).await.unwrap_err();
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
    });
}
