//! Stateless composition over guest-owned keystroke tokens. No host IME state.
use alloc::vec::Vec;

const INITIAL: &[u16] = &[
    0x3131, 0x3132, 0x3134, 0x3137, 0x3138, 0x3139, 0x3141, 0x3142, 0x3143, 0x3145, 0x3146, 0x3147, 0x3148, 0x3149, 0x314a, 0x314b, 0x314c, 0x314d,
    0x314e,
];
const FINAL: &[u16] = &[
    0, 0x3131, 0x3132, 0x3133, 0x3134, 0x3135, 0x3136, 0x3137, 0x3139, 0x313a, 0x313b, 0x313c, 0x313d, 0x313e, 0x313f, 0x3140, 0x3141, 0x3142,
    0x3144, 0x3145, 0x3146, 0x3147, 0x3148, 0x314a, 0x314b, 0x314c, 0x314d, 0x314e,
];
const PAIRS: &[(u16, u16, u16)] = &[
    (1, 19, 3),
    (4, 22, 5),
    (4, 27, 6),
    (8, 1, 9),
    (8, 16, 10),
    (8, 17, 11),
    (8, 19, 12),
    (8, 25, 13),
    (8, 26, 14),
    (8, 27, 15),
    (17, 19, 18),
];
// Vowel components: 1=ㅣ, 2=ㆍ, 3=ㅡ. Values index modern Hangul vowels.
const VOWELS: &[(&[u16], u16)] = &[
    (&[1, 2], 0),
    (&[1, 2, 1], 1),
    (&[1, 2, 2], 2),
    (&[1, 2, 2, 1], 3),
    (&[2, 1], 4),
    (&[2, 1, 1], 5),
    (&[2, 2, 1], 6),
    (&[2, 2, 1, 1], 7),
    (&[2, 3], 8),
    (&[2, 3, 1, 2], 9),
    (&[2, 3, 1, 2, 1], 10),
    (&[2, 3, 1], 11),
    (&[2, 2, 3], 12),
    (&[3, 2], 13),
    (&[3, 2, 2, 1], 14),
    (&[3, 2, 2, 1, 1], 15),
    (&[3, 2, 1], 16),
    (&[3, 2, 2], 17),
    (&[3], 18),
    (&[3, 1], 19),
    (&[1], 20),
];

pub fn push(tokens: &mut Vec<u16>, digit: u16, cycle: bool) {
    let group: &[u16] = match digit {
        0 => &[0x3147, 0x3141],
        4 => &[0x3131, 0x314b, 0x3132],
        5 => &[0x3134, 0x3139],
        6 => &[0x3137, 0x314c, 0x3138],
        7 => &[0x3142, 0x314d, 0x3143],
        8 => &[0x3145, 0x314e, 0x3146],
        9 => &[0x3148, 0x314a, 0x3149],
        1..=3 => {
            tokens.push(digit);
            return;
        }
        _ => return,
    };
    if cycle
        && let Some(last) = tokens.last_mut()
        && let Some(index) = group.iter().position(|c| c == last)
    {
        *last = group[(index + 1) % group.len()];
        return;
    }
    tokens.push(group[0]);
}

pub fn compose(tokens: &[u16]) -> Vec<u16> {
    let mut jamo = Vec::new();
    let mut i = 0;
    while i < tokens.len() {
        if tokens[i] > 3 {
            jamo.push(tokens[i]);
            i += 1;
            continue;
        }
        let best = VOWELS
            .iter()
            .filter(|(keys, _)| tokens[i..].starts_with(keys))
            .max_by_key(|(keys, _)| keys.len());
        if let Some((keys, v)) = best {
            jamo.push(0x314f + v);
            i += keys.len();
        } else {
            jamo.push(0x318d);
            i += 1;
        }
    }
    let mut out: Vec<u16> = Vec::new();
    for c in jamo {
        if (0x314f..=0x3163).contains(&c) {
            let v = c - 0x314f;
            if let Some(last) = out.last_mut() {
                if let Some(l) = INITIAL.iter().position(|x| *x == *last) {
                    *last = 0xac00 + l as u16 * 588 + v * 28;
                    continue;
                }
                if (0xac00..=0xd7a3).contains(last) && (*last - 0xac00) % 28 != 0 {
                    let t = (*last - 0xac00) % 28;
                    let (keep, carry) = PAIRS.iter().find(|(_, _, sum)| *sum == t).map(|(a, b, _)| (*a, *b)).unwrap_or((0, t));
                    if let Some(l) = INITIAL.iter().position(|x| *x == FINAL[carry as usize]) {
                        *last = *last - t + keep;
                        out.push(0xac00 + l as u16 * 588 + v * 28);
                        continue;
                    }
                }
            }
        } else if let Some(t) = FINAL.iter().position(|x| *x == c)
            && let Some(last) = out.last_mut()
            && (0xac00..=0xd7a3).contains(last)
        {
            let prior = (*last - 0xac00) % 28;
            if prior == 0 {
                *last += t as u16;
                continue;
            }
            if let Some((_, _, combined)) = PAIRS.iter().find(|(a, b, _)| *a == prior && *b == t as u16) {
                *last = *last - prior + combined;
                continue;
            }
        }
        out.push(c);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    fn text(tokens: &[u16]) -> alloc::string::String {
        alloc::string::String::from_utf16(&compose(tokens)).unwrap()
    }
    #[test]
    fn every_modern_vowel() {
        for (keys, v) in VOWELS {
            assert_eq!(compose(keys), [0x314f + v]);
        }
    }
    #[test]
    fn syllables_final_clusters_and_resyllabification() {
        assert_eq!(text(&[0x314e, 1, 2, 0x3134, 0x3131, 3, 0x3139]), "한글");
        assert_eq!(text(&[0x3131, 1, 2, 0x3131, 1, 2]), "가가");
        assert_eq!(text(&[0x3131, 1, 2, 0x3142, 0x3145, 1]), "갑시");
        assert_eq!(text(&[0x3131, 1, 2, 0x3142, 0x3145, 0x3147, 1]), "값이");
        assert_eq!(text(&[0x3131, 0x3131]), "ㄱㄱ");
    }
    #[test]
    fn consonant_cycles_and_component_backspace() {
        let mut tokens = Vec::new();
        push(&mut tokens, 4, false);
        push(&mut tokens, 4, true);
        assert_eq!(text(&tokens), "ㅋ");
        push(&mut tokens, 4, true);
        assert_eq!(text(&tokens), "ㄲ");
        push(&mut tokens, 1, false);
        push(&mut tokens, 2, false);
        assert_eq!(text(&tokens), "까");
        tokens.pop();
        assert_eq!(text(&tokens), "끼");
        tokens.pop();
        assert_eq!(text(&tokens), "ㄲ");
    }
}
