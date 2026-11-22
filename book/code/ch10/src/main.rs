//! 10 章「ライフタイム注釈」の実例。
//!
//! 引数から借りた `&str` を返す関数と、`&str` をしまう構造体を書く。
//! 9 章までは省略規則で済んでいた形を外れ、戻り値がどの引数から借りたかを注釈で書く。
//! 本物の取り出しは `crates/tally-core/src/select.rs` にある。
//!
//! 1 章から 9 章のコードとは独立している。章を飛ばして読んでも動く。

#[cfg(test)]
mod exercises;

/// 行の最初の欄。
///
/// 引数の参照が 1 つなので、戻り値がそこから借りることは省略規則で決まる。
fn first_field(line: &str) -> &str {
    line.split_whitespace().next().unwrap_or("")
}

/// 長いほうを返す。同じ長さなら `a`。
///
/// どちらを返すかは実行するまで分からないので、両方を同じ `'a` で結ぶ。
fn longer<'a>(a: &'a str, b: &'a str) -> &'a str {
    if a.len() >= b.len() { a } else { b }
}

/// 空でない行だけを残す。
///
/// 結ぶのは要素の `&str` だけで、スライスそのものとは結ばない。
/// 両方を `'a` で結ぶと、呼び手はスライスを結果より長く持ち続けなければならない。
fn non_empty<'a>(lines: &[&'a str]) -> Vec<&'a str> {
    lines
        .iter()
        .copied()
        .filter(|line| !line.is_empty())
        .collect()
}

/// 状態コードの説明。
///
/// 引数に参照が無いので、借りてくる先が無い。返せるのはプログラムに埋め込まれた文字列だけである。
fn describe(code: u16) -> &'static str {
    match code {
        200..=299 => "成功",
        400..=499 => "要求の誤り",
        500..=599 => "サーバの失敗",
        _ => "その他",
    }
}

/// `name=value` の形の欄から、名前が `name` の値を取り出す。
///
/// 名前は `String` を持たず、呼び手の文字列を借りる。
#[derive(Debug)]
struct Field<'n> {
    name: &'n str,
}

impl Field<'_> {
    /// 戻り値は `line` から借りる。`self` から借りるのではないことを `'l` で書く。
    ///
    /// 書かなければ省略規則が戻り値を `self` に結び、この本体は通らない。
    fn pick<'l>(&self, line: &'l str) -> Option<&'l str> {
        line.split_whitespace()
            .filter_map(|pair| pair.split_once('='))
            .find(|&(name, _)| name == self.name)
            .map(|(_, value)| value)
    }
}

fn main() {
    let lines = ["200 GET /", "", "404 GET /x", "", "503 POST /y"];
    for line in non_empty(&lines) {
        let code = first_field(line);
        println!("{code} {}", describe(code.parse().unwrap_or(0)));
    }
    println!("{}", longer("warning", "error"));

    let line = String::from("lvl=warn msg=disk");
    let value;
    {
        let name = String::from("lvl");
        let field = Field { name: &name };
        value = field.pick(&line);
        println!("{field:?}");
    }
    // `name` と `field` はもう無い。`value` が借りているのは `line` だけなので使える。
    println!("{value:?}");
}

#[cfg(test)]
mod tests {
    use super::{Field, describe, first_field, longer, non_empty};

    #[test]
    fn 省略規則で最初の欄を借りて返す() {
        assert_eq!(first_field("404 GET /x"), "404");
        assert_eq!(first_field("   "), "");
    }

    #[test]
    fn 同じ長さなら前の引数を返す() {
        assert_eq!(longer("warning", "error"), "warning");
        assert_eq!(longer("info", "warn"), "info");
        assert_eq!(longer("", "x"), "x");
    }

    #[test]
    fn 短いほうの寿命の内側でなら結果を使える() {
        let a = String::from("warning");
        let n;
        {
            let b = String::from("error");
            n = longer(&a, &b).len();
        }
        assert_eq!(n, 7);
    }

    #[test]
    fn スライスが消えても選んだ行は使える() {
        let text = String::from("a\n\nb\n");
        let kept;
        {
            let lines: Vec<&str> = text.lines().collect();
            kept = non_empty(&lines);
        }
        assert_eq!(kept, vec!["a", "b"]);
        assert!(non_empty(&[]).is_empty());
    }

    #[test]
    fn 状態コードの範囲の境目() {
        assert_eq!(describe(200), "成功");
        assert_eq!(describe(299), "成功");
        assert_eq!(describe(300), "その他");
        assert_eq!(describe(404), "要求の誤り");
        assert_eq!(describe(599), "サーバの失敗");
        assert_eq!(describe(0), "その他");
    }

    #[test]
    fn 名前が一致する欄だけを取り出す() {
        let field = Field { name: "lvl" };
        assert_eq!(field.pick("lvl=warn msg=disk"), Some("warn"));
        assert_eq!(field.pick("level=warn lv=x"), None);
        assert_eq!(field.pick("lvl msg=disk"), None);
        assert_eq!(field.pick("lvl= msg=disk"), Some(""));
    }

    #[test]
    fn 取り出した値は_field_より長く使える() {
        let line = String::from("msg=disk lvl=error");
        let value;
        {
            let name = String::from("lvl");
            value = Field { name: &name }.pick(&line);
        }
        assert_eq!(value, Some("error"));
    }

    /// 共有参照の先では、長い寿命の参照を短い寿命の参照として渡せる。
    fn first_or<'a>(items: &[&'a str], fallback: &'a str) -> &'a str {
        items.first().copied().unwrap_or(fallback)
    }

    #[test]
    fn static_の参照は短い寿命の参照として渡せる() {
        let fixed: Vec<&'static str> = vec!["warn"];
        let local = String::from("error");
        assert_eq!(first_or(&fixed, &local), "warn");
        assert_eq!(first_or(&[], &local), "error");
        assert_eq!(longer("error", &local), "error");
    }
}
