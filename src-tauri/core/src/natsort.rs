//! Natural ordering: "Episode 2" before "Episode 10", case-insensitive,
//! digits in any script (Persian ۱۲ and Arabic-Indic ١٢ count as numbers).

use std::cmp::Ordering;

fn digit(c: char) -> Option<u32> {
    match c {
        '0'..='9' => Some(c as u32 - '0' as u32),
        '\u{06F0}'..='\u{06F9}' => Some(c as u32 - 0x06F0),
        '\u{0660}'..='\u{0669}' => Some(c as u32 - 0x0660),
        _ => None,
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Part {
    Num(u128, usize), // value, digit count (to keep "01" before "1" stable)
    Text(String),
}

fn parts(s: &str) -> Vec<Part> {
    let mut out = Vec::new();
    let mut it = s.chars().peekable();
    while let Some(&c) = it.peek() {
        if digit(c).is_some() {
            let mut v: u128 = 0;
            let mut n = 0;
            while let Some(&d) = it.peek() {
                match digit(d) {
                    Some(x) => {
                        v = v.saturating_mul(10).saturating_add(x as u128);
                        n += 1;
                        it.next();
                    }
                    None => break,
                }
            }
            out.push(Part::Num(v, n));
        } else {
            let mut t = String::new();
            while let Some(&d) = it.peek() {
                if digit(d).is_some() {
                    break;
                }
                t.extend(d.to_lowercase());
                it.next();
            }
            out.push(Part::Text(t));
        }
    }
    out
}

pub fn compare(a: &str, b: &str) -> Ordering {
    let (pa, pb) = (parts(a), parts(b));
    for (x, y) in pa.iter().zip(pb.iter()) {
        let o = match (x, y) {
            (Part::Num(a, na), Part::Num(b, nb)) => a.cmp(b).then(nb.cmp(na)),
            (Part::Text(a), Part::Text(b)) => a.cmp(b),
            (Part::Num(..), Part::Text(_)) => Ordering::Less,
            (Part::Text(_), Part::Num(..)) => Ordering::Greater,
        };
        if o != Ordering::Equal {
            return o;
        }
    }
    pa.len().cmp(&pb.len()).then_with(|| a.cmp(b))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn orders_numbers_naturally() {
        let mut v = vec!["Ep 10.mkv", "Ep 2.mkv", "ep 1.mkv", "Ep 1b.mkv", "Ep 01.mkv"];
        v.sort_by(|a, b| compare(a, b));
        assert_eq!(v, vec!["Ep 01.mkv", "ep 1.mkv", "Ep 1b.mkv", "Ep 2.mkv", "Ep 10.mkv"]);
    }
    #[test]
    fn persian_digits() {
        let mut v = vec!["قسمت ۱۰", "قسمت ۲", "قسمت ۱"];
        v.sort_by(|a, b| compare(a, b));
        assert_eq!(v, vec!["قسمت ۱", "قسمت ۲", "قسمت ۱۰"]);
    }
    #[test]
    fn seasons_and_episodes() {
        let mut v = vec!["Show.S02E01.mkv", "Show.S01E10.mkv", "Show.S01E02.mkv", "Show.S01E01.mkv"];
        v.sort_by(|a, b| compare(a, b));
        assert_eq!(v, vec!["Show.S01E01.mkv", "Show.S01E02.mkv", "Show.S01E10.mkv", "Show.S02E01.mkv"]);
    }
}
