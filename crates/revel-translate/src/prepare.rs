//! Préparation d'un modèle téléchargé ([`install`](crate::install)) : poids d'un modèle OPUS-MT
//! de Hugging Face (`model.safetensors`, sinon `flax_model.msgpack`)
//! réduits aux tenseurs utiles et écrits en 16 bits (taille divisée par deux), avec la
//! configuration, le découpage et le vocabulaire.

use crate::TranslateError;
use candle_core::{DType, Device, Tensor};
use std::collections::HashMap;
use std::path::Path;

/// Prépare le modèle du dossier `src` dans `out` ; `source` : origine écrite dans la notice
/// (dépôt et version).
pub fn prepare(src: &Path, out: &Path, source: &str) -> Result<(), TranslateError> {
    let bad = |m: String| TranslateError::Model(m);
    let err = |e: candle_core::Error| TranslateError::Model(e.to_string());
    let device = Device::Cpu;
    let st = src.join("model.safetensors");
    let mut tensors: HashMap<String, Tensor> = if st.is_file() {
        candle_core::safetensors::load(&st, &device).map_err(err)?
    } else {
        // Sans `model.safetensors` : poids Flax (`flax_model.msgpack`), lus sans Python.
        let fx = src.join("flax_model.msgpack");
        let bytes = std::fs::read(&fx).map_err(|e| bad(format!("{} : {e}", fx.display())))?;
        flax_tensors(&bytes, &device)?
    };
    if !tensors.contains_key("model.shared.weight") {
        let shared = [
            "model.decoder.embed_tokens.weight",
            "model.encoder.embed_tokens.weight",
        ]
        .iter()
        .find_map(|k| tensors.get(*k).cloned())
        .ok_or_else(|| bad("plongements absents".into()))?;
        tensors.insert("model.shared.weight".into(), shared);
    }
    // Positions calculées par le modèle, plongements communs : copies inutiles retirées.
    tensors.retain(|k, _| {
        !k.ends_with("embed_positions.weight")
            && k != "model.decoder.embed_tokens.weight"
            && k != "model.encoder.embed_tokens.weight"
            && k != "lm_head.weight"
    });
    let half = tensors
        .into_iter()
        .map(|(k, t)| Ok((k, t.to_dtype(DType::F16)?)))
        .collect::<candle_core::Result<HashMap<_, _>>>()
        .map_err(err)?;
    std::fs::create_dir_all(out).map_err(|e| bad(format!("{} : {e}", out.display())))?;
    let weights = out.join("model.safetensors");
    candle_core::safetensors::save(&half, &weights).map_err(err)?;
    // Fichier lisible par tous (installé par l'administrateur, lu par chaque utilisateur).
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&weights, std::fs::Permissions::from_mode(0o644))
            .map_err(|e| bad(e.to_string()))?;
    }
    for f in ["config.json", "source.spm", "vocab.json"] {
        std::fs::copy(src.join(f), out.join(f))
            .map_err(|e| bad(format!("{} : {e}", src.join(f).display())))?;
    }
    std::fs::write(
        out.join("NOTICE.txt"),
        format!(
            "Modèle de traduction OPUS-MT (Helsinki-NLP), licence Apache-2.0 :\n\
             https://www.apache.org/licenses/LICENSE-2.0\n\
             Source : {source}\n\
             Poids convertis en 16 bits pour Revel (aucun autre changement).\n"
        ),
    )
    .map_err(|e| bad(e.to_string()))?;
    Ok(())
}

/// Valeur MessagePack (sous-ensemble utile aux poids Flax).
enum Mp<'a> {
    Map(Vec<(String, Mp<'a>)>),
    Arr(Vec<Mp<'a>>),
    Str(String),
    Bin(&'a [u8]),
    Int(i64),
    Ext(i8, &'a [u8]),
    Other,
}

struct Reader<'a> {
    b: &'a [u8],
    i: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Option<&'a [u8]> {
        let s = self.b.get(self.i..self.i.checked_add(n)?)?;
        self.i += n;
        Some(s)
    }
    fn uint(&mut self, n: usize) -> Option<u64> {
        Some(
            self.take(n)?
                .iter()
                .fold(0u64, |a, x| (a << 8) | u64::from(*x)),
        )
    }
    fn len(&mut self, n: usize) -> Option<usize> {
        usize::try_from(self.uint(n)?).ok()
    }
    fn str(&mut self, n: usize) -> Option<Mp<'a>> {
        Some(Mp::Str(String::from_utf8(self.take(n)?.to_vec()).ok()?))
    }
    fn map(&mut self, n: usize) -> Option<Mp<'a>> {
        let mut v = Vec::with_capacity(n.min(1024));
        for _ in 0..n {
            let k = match self.value()? {
                Mp::Str(s) => s,
                Mp::Int(i) => i.to_string(),
                _ => return None,
            };
            v.push((k, self.value()?));
        }
        Some(Mp::Map(v))
    }
    fn arr(&mut self, n: usize) -> Option<Mp<'a>> {
        let mut v = Vec::with_capacity(n.min(1024));
        for _ in 0..n {
            v.push(self.value()?);
        }
        Some(Mp::Arr(v))
    }
    fn ext(&mut self, n: usize) -> Option<Mp<'a>> {
        let code = *self.take(1)?.first()? as i8;
        Some(Mp::Ext(code, self.take(n)?))
    }
    fn value(&mut self) -> Option<Mp<'a>> {
        let t = *self.take(1)?.first()?;
        Some(match t {
            0x00..=0x7f => Mp::Int(i64::from(t)),
            0x80..=0x8f => return self.map(usize::from(t & 0x0f)),
            0x90..=0x9f => return self.arr(usize::from(t & 0x0f)),
            0xa0..=0xbf => return self.str(usize::from(t & 0x1f)),
            0xc0 | 0xc2 | 0xc3 => Mp::Other,
            0xc4..=0xc6 => {
                let n = self.len(1 << (t - 0xc4))?;
                Mp::Bin(self.take(n)?)
            }
            0xc7..=0xc9 => {
                let n = self.len(1 << (t - 0xc7))?;
                return self.ext(n);
            }
            0xca => {
                self.take(4)?;
                Mp::Other
            }
            0xcb => {
                self.take(8)?;
                Mp::Other
            }
            0xcc..=0xcf => Mp::Int(i64::try_from(self.uint(1 << (t - 0xcc))?).ok()?),
            0xd0..=0xd3 => {
                let n = 1usize << (t - 0xd0);
                let u = self.uint(n)?;
                let bits = 64 - 8 * n as u32;
                Mp::Int(((u << bits) as i64) >> bits)
            }
            0xd4..=0xd8 => return self.ext(1 << (t - 0xd4)),
            0xd9..=0xdb => {
                let n = self.len(1 << (t - 0xd9))?;
                return self.str(n);
            }
            0xdc | 0xdd => {
                let n = self.len(if t == 0xdc { 2 } else { 4 })?;
                return self.arr(n);
            }
            0xde | 0xdf => {
                let n = self.len(if t == 0xde { 2 } else { 4 })?;
                return self.map(n);
            }
            0xe0..=0xff => Mp::Int(i64::from(t as i8)),
            _ => Mp::Other,
        })
    }
}

/// Tenseurs d'un fichier de poids Flax (`flax.serialization` : tableaux en extension 1 =
/// `(forme, type, octets)`), renommés comme ceux de PyTorch (`kernel` transposé -> `weight`,
/// `scale` -> `weight`, `embedding` -> `weight`).
fn flax_tensors(bytes: &[u8], device: &Device) -> Result<HashMap<String, Tensor>, TranslateError> {
    let bad = |m: &str| TranslateError::Model(format!("flax_model.msgpack : {m}"));
    let err = |e: candle_core::Error| TranslateError::Model(e.to_string());
    let root = Reader { b: bytes, i: 0 }
        .value()
        .ok_or_else(|| bad("illisible"))?;
    let mut out = HashMap::new();
    let mut stack: Vec<(String, Mp)> = vec![(String::new(), root)];
    while let Some((path, v)) = stack.pop() {
        match v {
            Mp::Map(m) => {
                for (k, x) in m {
                    let p = if path.is_empty() {
                        k
                    } else {
                        format!("{path}.{k}")
                    };
                    stack.push((p, x));
                }
            }
            Mp::Ext(1, data) => {
                let Some(Mp::Arr(parts)) = (Reader { b: data, i: 0 }).value() else {
                    return Err(bad("tableau illisible"));
                };
                let [Mp::Arr(shape), Mp::Str(dtype), Mp::Bin(buf)] = parts.as_slice() else {
                    return Err(bad("tableau illisible"));
                };
                if dtype != "float32" {
                    return Err(bad(&format!("type {dtype}")));
                }
                let shape: Vec<usize> = shape
                    .iter()
                    .map(|d| match d {
                        Mp::Int(n) => usize::try_from(*n).ok(),
                        _ => None,
                    })
                    .collect::<Option<_>>()
                    .ok_or_else(|| bad("forme illisible"))?;
                let mut t =
                    Tensor::from_raw_buffer(buf, DType::F32, &shape, device).map_err(err)?;
                let name = if let Some(base) = path.strip_suffix(".kernel") {
                    t = t.t().and_then(|x| x.contiguous()).map_err(err)?;
                    format!("{base}.weight")
                } else if let Some(base) = path.strip_suffix(".scale") {
                    format!("{base}.weight")
                } else if let Some(base) = path.strip_suffix(".embedding") {
                    format!("{base}.weight")
                } else {
                    path
                };
                out.insert(name, t);
            }
            _ => {}
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn messagepack() {
        // {"a": {"b": ext 1 [[2], "float32", <8 octets>]}}
        let mut arr = vec![0x93, 0x91, 0x02, 0xa7];
        arr.extend_from_slice(b"float32");
        arr.extend_from_slice(&[0xc4, 0x08]);
        arr.extend_from_slice(&1f32.to_le_bytes());
        arr.extend_from_slice(&2f32.to_le_bytes());
        let mut b = vec![0x81, 0xa1, b'a', 0x81, 0xa5];
        b.extend_from_slice(b"scale");
        b.extend_from_slice(&[0xc7, arr.len() as u8, 0x01]);
        b.extend_from_slice(&arr);
        let t = flax_tensors(&b, &Device::Cpu).unwrap();
        let w = t.get("a.weight").unwrap();
        assert_eq!(w.to_vec1::<f32>().unwrap(), vec![1.0, 2.0]);
    }
}
