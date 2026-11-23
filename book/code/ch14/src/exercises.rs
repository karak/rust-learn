//! 14 章の演習の答え。
//!
//! `cargo test -p book-ch14` で確かめられる。

/// 演習 1: 1 以上 65535 以下のポート番号だけを表す検証つき newtype。
///
/// フィールドを非公開にし、0 を `new` で弾くことで、構築後の値には 0 が混ざらない。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Port(u16);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ZeroPortError;

impl Port {
    fn new(raw: u16) -> Result<Self, ZeroPortError> {
        if raw == 0 {
            Err(ZeroPortError)
        } else {
            Ok(Self(raw))
        }
    }

    fn get(self) -> u16 {
        self.0
    }
}

/// 演習 2: 閉じている状態のバッファ。開かないと書き込めない。
#[derive(Debug, Default)]
struct ClosedBuffer {
    lines: Vec<String>,
}

/// 演習 2: 開いている状態のバッファ。書き込んだあと `close` で閉じると結果を読める。
#[derive(Debug)]
struct OpenBuffer {
    lines: Vec<String>,
}

impl ClosedBuffer {
    fn new() -> Self {
        Self::default()
    }

    fn open(self) -> OpenBuffer {
        OpenBuffer { lines: self.lines }
    }

    fn lines(&self) -> &[String] {
        &self.lines
    }
}

impl OpenBuffer {
    fn push(&mut self, line: &str) {
        self.lines.push(line.to_owned());
    }

    fn close(self) -> ClosedBuffer {
        ClosedBuffer { lines: self.lines }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn port_は_0_を弾き正の数を受け入れる() {
        assert_eq!(Port::new(0), Err(ZeroPortError));
        let port = Port::new(8080).expect("1 以上");
        assert_eq!(port.get(), 8080);
    }

    #[test]
    fn typestate_のバッファは開いて書き閉じて読める() {
        let mut buf = ClosedBuffer::new().open();
        buf.push("info");
        buf.push("warn");
        let closed = buf.close();
        assert_eq!(closed.lines(), &["info", "warn"]);
    }
}
