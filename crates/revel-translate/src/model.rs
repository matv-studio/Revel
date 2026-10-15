//! Modèle de traduction Marian (OPUS-MT) exécuté sur le processeur avec `candle`.
//!
//! Dossier d'un modèle : `config.json`, `model.safetensors` (poids, 16 ou 32 bits), `source.spm`
//! (découpage du texte d'origine), `vocab.json` (morceaux -> numéros, commun aux deux langues).

use crate::TranslateError;
use crate::spm::{self, Unigram};
use candle_core::{DType, Device, IndexOp, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::marian::{Config, MTModel};
use std::collections::HashMap;
use std::path::Path;

/// Modèle chargé : réseau, découpage, vocabulaire.
pub struct Model {
    net: MTModel,
    cfg: Config,
    spm: Unigram,
    vocab: HashMap<String, u32>,
    pieces: Vec<String>,
    unk: u32,
    device: Device,
}

fn read(dir: &Path, name: &str) -> Result<Vec<u8>, TranslateError> {
    let p = dir.join(name);
    std::fs::read(&p).map_err(|e| TranslateError::Model(format!("{} : {e}", p.display())))
}

impl Model {
    /// Charge le modèle d'un dossier.
    pub fn open(dir: &Path) -> Result<Self, TranslateError> {
        let bad = |m: String| TranslateError::Model(m);
        let mut raw: serde_json::Value = serde_json::from_slice(&read(dir, "config.json")?)
            .map_err(|e| bad(format!("config.json : {e}")))?;
        // Configurations anciennes (opus-mt-en-de, opus-mt-en-zh) : valeur par défaut de
        // Hugging Face (plongements communs à l'encodeur et au décodeur).
        if let Some(o) = raw.as_object_mut() {
            o.entry("share_encoder_decoder_embeddings")
                .or_insert(serde_json::Value::Bool(true));
        }
        let cfg: Config =
            serde_json::from_value(raw).map_err(|e| bad(format!("config.json : {e}")))?;
        let spm = Unigram::from_bytes(&read(dir, "source.spm")?)
            .ok_or_else(|| bad("source.spm illisible".into()))?;
        let vocab: HashMap<String, u32> = serde_json::from_slice(&read(dir, "vocab.json")?)
            .map_err(|e| bad(format!("vocab.json : {e}")))?;
        let size = vocab.values().copied().max().unwrap_or(0) as usize + 1;
        let mut pieces = vec![String::new(); size];
        for (k, v) in &vocab {
            pieces[*v as usize] = k.clone();
        }
        let unk = vocab.get("<unk>").copied().unwrap_or(1);
        let device = Device::Cpu;
        let weights = dir.join("model.safetensors");
        let tensors = candle_core::safetensors::load(&weights, &device)
            .map_err(|e| bad(format!("{} : {e}", weights.display())))?;
        // Plongements communs : rangés sous le nom du décodeur dans les fichiers de Hugging Face.
        let mut tensors = tensors;
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
        // Poids gardés en 16 bits sur le disque, calculs en 32 bits.
        let tensors = tensors
            .into_iter()
            .map(|(k, t)| Ok((k, t.to_dtype(DType::F32)?)))
            .collect::<candle_core::Result<HashMap<_, _>>>()
            .map_err(|e| bad(e.to_string()))?;
        let vb = VarBuilder::from_tensors(tensors, DType::F32, &device);
        let net = MTModel::new(&cfg, vb).map_err(|e| bad(e.to_string()))?;
        Ok(Self {
            net,
            cfg,
            spm,
            vocab,
            pieces,
            unk,
            device,
        })
    }

    /// Traduit une phrase (texte sans balises), recherche en faisceau de largeur `beams`
    /// (1 : le mot le plus probable à chaque pas). `token` : jeton de la langue visée, pour les
    /// modèles qui en produisent plusieurs (`>>cmn_Hans<<`).
    pub fn sentence(
        &mut self,
        text: &str,
        beams: usize,
        token: Option<&str>,
    ) -> Result<String, TranslateError> {
        let err = |e: candle_core::Error| TranslateError::Model(e.to_string());
        let beams = beams.max(1);
        // Apostrophes typographiques : rares dans les textes d'apprentissage (« d’unique » donnait
        // « d »), remplacées par l'apostrophe droite.
        let text = text.replace(['\u{2019}', '\u{2018}', '\u{02BC}'], "'");
        let mut ids: Vec<u32> = self
            .spm
            .encode(&text)
            .iter()
            .map(|p| self.vocab.get(p).copied().unwrap_or(self.unk))
            .collect();
        if ids.is_empty() {
            return Ok(String::new());
        }
        if let Some(id) = token.and_then(|t| self.vocab.get(t)) {
            ids.insert(0, *id);
        }
        // Au-delà de la longueur maximale du modèle : coupé (le découpage en phrases l'évite).
        ids.truncate(self.cfg.max_position_embeddings.saturating_sub(2));
        ids.push(self.cfg.eos_token_id);
        let input = Tensor::new(ids.as_slice(), &self.device)
            .and_then(|t| t.unsqueeze(0))
            .map_err(err)?;
        self.net.reset_kv_cache();
        let encoded = self.net.encoder().forward(&input, 0).map_err(err)?;
        let max = (ids.len() * 2 + 16).min(self.cfg.max_position_embeddings - 1);
        // Faisceau : chaque piste garde sa copie du décodeur (mémoire des pas précédents).
        struct Track {
            net: MTModel,
            out: Vec<u32>,
            score: f32,
        }
        let mut tracks = vec![Track {
            net: self.net.clone(),
            out: vec![self.cfg.decoder_start_token_id],
            score: 0.0,
        }];
        let mut done: Vec<(Vec<u32>, f32)> = Vec::new();
        let eos = [self.cfg.eos_token_id, self.cfg.forced_eos_token_id];
        for step in 0..max {
            // Candidats : (piste, mot, score total).
            let mut cands: Vec<(usize, u32, f32)> = Vec::new();
            for (ti, t) in tracks.iter_mut().enumerate() {
                let ctx = if step == 0 { t.out.len() } else { 1 };
                let start = t.out.len() - ctx;
                let x = Tensor::new(&t.out[start..], &self.device)
                    .and_then(|x| x.unsqueeze(0))
                    .map_err(err)?;
                let logits = t.net.decode(&x, &encoded, start).map_err(err)?;
                let logits = logits.squeeze(0).map_err(err)?;
                let last = logits.i(logits.dim(0).map_err(err)? - 1).map_err(err)?;
                let mut v: Vec<f32> = last.to_vec1().map_err(err)?;
                // Jamais le jeton de remplissage (`bad_words_ids` du modèle).
                if let Some(p) = v.get_mut(self.cfg.pad_token_id as usize) {
                    *p = f32::NEG_INFINITY;
                }
                log_softmax(&mut v);
                for (tok, lp) in top_k(&v, beams) {
                    cands.push((ti, tok, t.score + lp));
                }
            }
            cands.sort_by(|a, b| b.2.total_cmp(&a.2));
            let mut next: Vec<Track> = Vec::with_capacity(beams);
            for (ti, tok, score) in cands {
                if next.len() + done.len() >= beams * 2 || next.len() >= beams {
                    break;
                }
                if eos.contains(&tok) {
                    done.push((tracks[ti].out.clone(), score));
                } else {
                    let mut out = tracks[ti].out.clone();
                    out.push(tok);
                    next.push(Track {
                        net: tracks[ti].net.clone(),
                        out,
                        score,
                    });
                }
            }
            tracks = next;
            if done.len() >= beams || tracks.is_empty() {
                break;
            }
        }
        // Pistes non terminées : candidates aussi.
        done.extend(tracks.into_iter().map(|t| (t.out, t.score)));
        self.net.reset_kv_cache();
        // Score rapporté à la longueur (sinon les traductions courtes l'emportent).
        let best = done
            .into_iter()
            .max_by(|a, b| {
                let na = a.1 / (a.0.len().max(1) as f32);
                let nb = b.1 / (b.0.len().max(1) as f32);
                na.total_cmp(&nb)
            })
            .map(|d| d.0)
            .unwrap_or_default();
        let pieces: Vec<&str> = best
            .iter()
            .skip(1)
            .filter_map(|i| self.pieces.get(*i as usize).map(String::as_str))
            .filter(|p| !p.starts_with('<') || !p.ends_with('>'))
            .collect();
        Ok(spm::decode(&pieces))
    }
}

/// Probabilités en logarithmes.
fn log_softmax(v: &mut [f32]) {
    let max = v.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let sum: f32 = v.iter().map(|x| (x - max).exp()).sum();
    let log = max + sum.ln();
    for x in v.iter_mut() {
        *x -= log;
    }
}

/// Les `k` meilleurs mots (numéro, score), du meilleur au moins bon.
fn top_k(v: &[f32], k: usize) -> Vec<(u32, f32)> {
    let mut best: Vec<(u32, f32)> = Vec::with_capacity(k + 1);
    for (i, &x) in v.iter().enumerate() {
        if x == f32::NEG_INFINITY || (best.len() == k && x <= best[k - 1].1) {
            continue;
        }
        let at = best.iter().position(|b| x > b.1).unwrap_or(best.len());
        best.insert(at, (i as u32, x));
        best.truncate(k);
    }
    best
}
