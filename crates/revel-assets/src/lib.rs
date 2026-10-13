//! `revel-assets` : images (AVIF/WebP/JPEG) et polices.
//!
//! Interface posée à l'étape 0 ; implémentation prévue : Étape 1.

/// Format d'image produit pour le web.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormat {
    Avif,
    WebP,
    Jpeg,
}

/// Largeurs générées par défaut (reprend le réglage du site Alunites : 480 à 2400 px).
pub const DEFAULT_WIDTHS: &[u32] = &[480, 800, 1200, 1600, 2400];

/// Variante d'une image source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Variant {
    pub path: String,
    pub format: ImageFormat,
    pub width: u32,
}

/// Chaîne de traitement des médias.
pub trait AssetPipeline {
    type Error: std::error::Error;
    /// Produit les variantes d'une image source (chemin relatif au site).
    fn image_variants(&self, source: &str) -> Result<Vec<Variant>, Self::Error>;
}

#[cfg(test)]
mod tests {
    #[test]
    fn largeurs_croissantes() {
        assert!(super::DEFAULT_WIDTHS.windows(2).all(|w| w[0] < w[1]));
    }
}
