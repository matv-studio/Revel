//! Découpage SentencePiece (modèle « unigram ») des modèles OPUS-MT, sans dépendance externe.
//!
//! Le fichier `source.spm` est un `ModelProto` (protobuf) : la liste des morceaux, chacun avec son
//! score (log-probabilité) et son type. Le texte est normalisé (NFKC, espaces regroupés, `▁` au
//! début et à la place des espaces), puis découpé par l'algorithme de Viterbi : la suite de
//! morceaux dont la somme des scores est la plus grande. Référence :
//! <https://github.com/google/sentencepiece/blob/master/src/sentencepiece_model.proto>.

use std::collections::HashMap;
use unicode_normalization::UnicodeNormalization;

/// Marque d'espace de SentencePiece.
pub const SPACE: char = '\u{2581}';

/// Modèle de découpage : morceaux utilisables et leur score.
#[derive(Debug, Clone)]
pub struct Unigram {
    scores: HashMap<String, f32>,
    /// Longueur maximale d'un morceau, en caractères.
    max_len: usize,
    /// Score donné à un caractère inconnu (plus bas que tout morceau).
    unk_score: f32,
}

/// Lecture d'un entier « varint » ; renvoie la valeur et la position suivante.
fn varint(b: &[u8], mut i: usize) -> Option<(u64, usize)> {
    let mut v = 0u64;
    let mut shift = 0;
    loop {
        let byte = *b.get(i)?;
        i += 1;
        v |= u64::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            return Some((v, i));
        }
        shift += 7;
        if shift > 63 {
            return None;
        }
    }
}

/// Champs d'un message protobuf : (numéro, type, contenu ou valeur).
enum Field<'a> {
    Varint(u64),
    Fixed32([u8; 4]),
    Bytes(&'a [u8]),
}

fn fields(b: &[u8]) -> Option<Vec<(u64, Field<'_>)>> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        let (key, j) = varint(b, i)?;
        i = j;
        let (num, wire) = (key >> 3, key & 7);
        match wire {
            0 => {
                let (v, j) = varint(b, i)?;
                i = j;
                out.push((num, Field::Varint(v)));
            }
            1 => i += 8,
            2 => {
                let (len, j) = varint(b, i)?;
                let end = j.checked_add(usize::try_from(len).ok()?)?;
                out.push((num, Field::Bytes(b.get(j..end)?)));
                i = end;
            }
            5 => {
                let v: [u8; 4] = b.get(i..i + 4)?.try_into().ok()?;
                out.push((num, Field::Fixed32(v)));
                i += 4;
            }
            _ => return None,
        }
    }
    Some(out)
}

impl Unigram {
    /// Lit un modèle SentencePiece (`source.spm`).
    pub fn from_bytes(b: &[u8]) -> Option<Self> {
        let mut scores = HashMap::new();
        let mut min = f32::MAX;
        for (num, f) in fields(b)? {
            let (1, Field::Bytes(piece)) = (num, f) else {
                continue;
            };
            let mut text = None;
            let mut score = 0f32;
            let mut kind = 1u64;
            for (n, g) in fields(piece)? {
                match (n, g) {
                    (1, Field::Bytes(s)) => text = Some(String::from_utf8(s.to_vec()).ok()?),
                    (2, Field::Fixed32(v)) => score = f32::from_le_bytes(v),
                    (3, Field::Varint(v)) => kind = v,
                    _ => {}
                }
            }
            // Morceaux ordinaires (1) et définis par l'utilisateur (4) seulement.
            if let Some(t) = text
                && (kind == 1 || kind == 4)
            {
                min = min.min(score);
                scores.insert(t, score);
            }
        }
        if scores.is_empty() {
            return None;
        }
        let max_len = scores.keys().map(|k| k.chars().count()).max().unwrap_or(1);
        Some(Self {
            scores,
            max_len,
            unk_score: min - 10.0,
        })
    }

    /// Texte normalisé comme SentencePiece (`nmt_nfkc`, espace initial, espaces marqués).
    pub fn normalize(text: &str) -> String {
        let nfkc: String = text.nfkc().collect();
        let words: Vec<&str> = nfkc.split_whitespace().collect();
        let mut out = String::with_capacity(nfkc.len() + 4);
        for w in words {
            out.push(SPACE);
            out.push_str(w);
        }
        out
    }

    /// Morceaux d'un texte (chemin de score maximal).
    pub fn encode(&self, text: &str) -> Vec<String> {
        let norm = Self::normalize(text);
        let chars: Vec<char> = norm.chars().collect();
        let n = chars.len();
        if n == 0 {
            return Vec::new();
        }
        // Positions en octets de chaque caractère (pour extraire les sous-chaînes).
        let mut at: Vec<usize> = norm.char_indices().map(|(i, _)| i).collect();
        at.push(norm.len());
        let mut best = vec![f32::NEG_INFINITY; n + 1];
        let mut from = vec![0usize; n + 1];
        best[0] = 0.0;
        for end in 1..=n {
            let lo = end.saturating_sub(self.max_len);
            for start in lo..end {
                if best[start] == f32::NEG_INFINITY {
                    continue;
                }
                let piece = &norm[at[start]..at[end]];
                let s = match self.scores.get(piece) {
                    Some(s) => *s,
                    // Caractère seul inconnu : gardé comme morceau inconnu.
                    None if end - start == 1 => self.unk_score,
                    None => continue,
                };
                if best[start] + s > best[end] {
                    best[end] = best[start] + s;
                    from[end] = start;
                }
            }
        }
        let mut pieces = Vec::new();
        let mut end = n;
        while end > 0 {
            let start = from[end];
            pieces.push(norm[at[start]..at[end]].to_string());
            end = start;
        }
        pieces.reverse();
        pieces
    }
}

/// Texte d'une suite de morceaux produits par le modèle.
pub fn decode(pieces: &[&str]) -> String {
    let s: String = pieces.concat();
    s.replace(SPACE, " ").trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalisation() {
        assert_eq!(
            Unigram::normalize("  Bonjour   le\nmonde "),
            "▁Bonjour▁le▁monde"
        );
        assert_eq!(Unigram::normalize("ﬁn"), "▁fin");
    }

    #[test]
    fn decoupage() {
        let u = Unigram {
            scores: [
                ("▁le", -1.0),
                ("▁", -2.0),
                ("le", -3.0),
                ("▁chat", -1.5),
                ("chat", -4.0),
            ]
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect(),
            max_len: 5,
            unk_score: -20.0,
        };
        assert_eq!(u.encode("le chat"), vec!["▁le", "▁chat"]);
        assert_eq!(u.encode("le X"), vec!["▁le", "▁", "X"]);
        assert_eq!(decode(&["▁le", "▁chat"]), "le chat");
    }
}
