//! Couleurs : lecture des notations CSS courantes, conversions RVB / TSV / TSL, contraste WCAG.

use super::split_top;

/// Couleur sRGB : composantes 0-255 (réelles, pour ne pas perdre de précision en glissant), alpha 0-1.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rgba {
    pub r: f64,
    pub g: f64,
    pub b: f64,
    pub a: f64,
}

/// Teinte (0-360), saturation et valeur (0-1), alpha (0-1) : modèle du sélecteur de couleur.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hsva {
    pub h: f64,
    pub s: f64,
    pub v: f64,
    pub a: f64,
}

fn clamp01(x: f64) -> f64 {
    x.clamp(0.0, 1.0)
}

/// Nombre ou pourcentage (`50%` -> 0.5 × `full`).
fn component(t: &str, full: f64) -> Option<f64> {
    let t = t.trim();
    if t.eq_ignore_ascii_case("none") {
        return Some(0.0);
    }
    match t.strip_suffix('%') {
        Some(p) => p.trim().parse::<f64>().ok().map(|v| v / 100.0 * full),
        None => t.parse::<f64>().ok(),
    }
}

/// Angle en degrés (`120`, `120deg`, `0.5turn`, `3rad`, `200grad`).
fn angle(t: &str) -> Option<f64> {
    let t = t.trim().to_ascii_lowercase();
    let (num, mul) = if let Some(n) = t.strip_suffix("deg") {
        (n, 1.0)
    } else if let Some(n) = t.strip_suffix("grad") {
        (n, 0.9)
    } else if let Some(n) = t.strip_suffix("rad") {
        (n, 180.0 / std::f64::consts::PI)
    } else if let Some(n) = t.strip_suffix("turn") {
        (n, 360.0)
    } else {
        (t.as_str(), 1.0)
    };
    num.trim()
        .parse::<f64>()
        .ok()
        .map(|v| (v * mul).rem_euclid(360.0))
}

/// Arguments d'une fonction de couleur : `1, 2, 3, .5` ou `1 2 3 / 50%`.
fn args(inner: &str) -> Option<(Vec<String>, Option<String>)> {
    if inner.contains(',') {
        let mut parts = split_top(inner, ',');
        let alpha = (parts.len() == 4).then(|| parts.pop()).flatten();
        return (parts.len() == 3).then_some((parts, alpha));
    }
    let (main, alpha) = match inner.split_once('/') {
        Some((m, a)) => (m, Some(a.trim().to_string())),
        None => (inner, None),
    };
    let parts = split_top(main, ' ');
    (parts.len() == 3).then_some((parts, alpha))
}

impl Rgba {
    pub const BLACK: Rgba = Rgba {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };
    pub const WHITE: Rgba = Rgba {
        r: 255.0,
        g: 255.0,
        b: 255.0,
        a: 1.0,
    };

    pub fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self {
            r: r.into(),
            g: g.into(),
            b: b.into(),
            a: 1.0,
        }
    }

    /// Lit `#rgb`, `#rgba`, `#rrggbb`, `#rrggbbaa`, `rgb()`/`rgba()`, `hsl()`/`hsla()`,
    /// `transparent` et les 148 noms de couleur. Autre chose (`var()`, `currentcolor`) -> `None`.
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim();
        let lower = s.to_ascii_lowercase();
        if let Some(hex) = lower.strip_prefix('#') {
            return Self::from_hex(hex);
        }
        if lower == "transparent" {
            return Some(Self {
                a: 0.0,
                ..Self::BLACK
            });
        }
        if let Some(inner) = lower
            .strip_prefix("rgba(")
            .or_else(|| lower.strip_prefix("rgb("))
            .and_then(|r| r.strip_suffix(')'))
        {
            let (p, alpha) = args(inner)?;
            return Some(Self {
                r: component(&p[0], 255.0)?.clamp(0.0, 255.0),
                g: component(&p[1], 255.0)?.clamp(0.0, 255.0),
                b: component(&p[2], 255.0)?.clamp(0.0, 255.0),
                a: match alpha {
                    Some(a) => clamp01(component(&a, 1.0)?),
                    None => 1.0,
                },
            });
        }
        if let Some(inner) = lower
            .strip_prefix("hsla(")
            .or_else(|| lower.strip_prefix("hsl("))
            .and_then(|r| r.strip_suffix(')'))
        {
            let (p, alpha) = args(inner)?;
            let pct = |t: &str| -> Option<f64> {
                let t = t.trim();
                t.strip_suffix('%')
                    .unwrap_or(t)
                    .trim()
                    .parse::<f64>()
                    .ok()
                    .map(|v| clamp01(v / 100.0))
            };
            let mut c = Self::from_hsl(angle(&p[0])?, pct(&p[1])?, pct(&p[2])?);
            if let Some(a) = alpha {
                c.a = clamp01(component(&a, 1.0)?);
            }
            return Some(c);
        }
        NAMED
            .iter()
            .find(|(n, _)| *n == lower)
            .map(|(_, v)| Self::rgb((v >> 16) as u8, (v >> 8) as u8, *v as u8))
    }

    fn from_hex(hex: &str) -> Option<Self> {
        if !hex.chars().all(|c| c.is_ascii_hexdigit()) {
            return None;
        }
        let d = |i: usize, n: usize| u8::from_str_radix(&hex[i..i + n], 16).ok();
        let short = |i: usize| d(i, 1).map(|v| v * 17);
        let (r, g, b, a) = match hex.len() {
            3 => (short(0)?, short(1)?, short(2)?, 255),
            4 => (short(0)?, short(1)?, short(2)?, short(3)?),
            6 => (d(0, 2)?, d(2, 2)?, d(4, 2)?, 255),
            8 => (d(0, 2)?, d(2, 2)?, d(4, 2)?, d(6, 2)?),
            _ => return None,
        };
        Some(Self {
            a: f64::from(a) / 255.0,
            ..Self::rgb(r, g, b)
        })
    }

    fn bytes(&self) -> (u8, u8, u8) {
        let q = |v: f64| v.round().clamp(0.0, 255.0) as u8;
        (q(self.r), q(self.g), q(self.b))
    }

    /// Alpha arrondi au centième.
    pub fn alpha(&self) -> f64 {
        (self.a * 100.0).round() / 100.0
    }

    /// `#rrggbb` (sans alpha).
    pub fn hex6(&self) -> String {
        let (r, g, b) = self.bytes();
        format!("#{r:02x}{g:02x}{b:02x}")
    }

    /// Écriture dans le CSS : `#rrggbb` si opaque, sinon `rgba(r, g, b, a)` (lu partout).
    pub fn to_css(&self) -> String {
        let a = self.alpha();
        if a >= 1.0 {
            return self.hex6();
        }
        let (r, g, b) = self.bytes();
        format!("rgba({r}, {g}, {b}, {})", super::fmt_num(a))
    }

    pub fn to_hsva(&self) -> Hsva {
        let (r, g, b) = (self.r / 255.0, self.g / 255.0, self.b / 255.0);
        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let d = max - min;
        let h = if d == 0.0 {
            0.0
        } else if max == r {
            60.0 * ((g - b) / d).rem_euclid(6.0)
        } else if max == g {
            60.0 * ((b - r) / d + 2.0)
        } else {
            60.0 * ((r - g) / d + 4.0)
        };
        Hsva {
            h,
            s: if max == 0.0 { 0.0 } else { d / max },
            v: max,
            a: self.a,
        }
    }

    pub fn from_hsva(c: Hsva) -> Self {
        let h = c.h.rem_euclid(360.0) / 60.0;
        let (s, v) = (clamp01(c.s), clamp01(c.v));
        let f = |n: f64| {
            let k = (n + h).rem_euclid(6.0);
            v - v * s * k.min(4.0 - k).clamp(0.0, 1.0)
        };
        Self {
            r: f(5.0) * 255.0,
            g: f(3.0) * 255.0,
            b: f(1.0) * 255.0,
            a: clamp01(c.a),
        }
    }

    /// Teinte (0-360), saturation et luminosité (0-1).
    pub fn to_hsl(&self) -> (f64, f64, f64) {
        let hsv = self.to_hsva();
        let l = hsv.v * (1.0 - hsv.s / 2.0);
        let s = if l == 0.0 || l == 1.0 {
            0.0
        } else {
            (hsv.v - l) / l.min(1.0 - l)
        };
        (hsv.h, s, l)
    }

    pub fn from_hsl(h: f64, s: f64, l: f64) -> Self {
        let (s, l) = (clamp01(s), clamp01(l));
        let v = l + s * l.min(1.0 - l);
        let sv = if v == 0.0 { 0.0 } else { 2.0 * (1.0 - l / v) };
        Self::from_hsva(Hsva {
            h,
            s: sv,
            v,
            a: 1.0,
        })
    }

    /// Luminance relative (WCAG 2).
    pub fn luminance(&self) -> f64 {
        let lin = |c: f64| {
            let c = c / 255.0;
            if c <= 0.04045 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * lin(self.r) + 0.7152 * lin(self.g) + 0.0722 * lin(self.b)
    }

    /// Couleur posée sur un fond opaque.
    pub fn over(&self, bg: Rgba) -> Rgba {
        let a = self.a;
        Rgba {
            r: self.r * a + bg.r * (1.0 - a),
            g: self.g * a + bg.g * (1.0 - a),
            b: self.b * a + bg.b * (1.0 - a),
            a: 1.0,
        }
    }
}

/// Rapport de contraste WCAG entre un texte et un fond (le texte est posé sur le fond).
pub fn contrast(fg: Rgba, bg: Rgba) -> f64 {
    let bg = bg.over(Rgba::WHITE);
    let fg = fg.over(bg);
    let (a, b) = (fg.luminance(), bg.luminance());
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}

/// Valeur qui désigne une couleur (notation reconnue, `currentcolor` ou variable).
pub fn is_color(token: &str) -> bool {
    Rgba::parse(token).is_some()
        || token.eq_ignore_ascii_case("currentcolor")
        || token.to_ascii_lowercase().starts_with("color-mix(")
        || ["oklch(", "oklab(", "lab(", "lch(", "hwb(", "color("]
            .iter()
            .any(|p| token.to_ascii_lowercase().starts_with(p))
}

/// Les 148 couleurs nommées de CSS Color 4.
const NAMED: &[(&str, u32)] = &[
    ("aliceblue", 0xf0f8ff),
    ("antiquewhite", 0xfaebd7),
    ("aqua", 0x00ffff),
    ("aquamarine", 0x7fffd4),
    ("azure", 0xf0ffff),
    ("beige", 0xf5f5dc),
    ("bisque", 0xffe4c4),
    ("black", 0x000000),
    ("blanchedalmond", 0xffebcd),
    ("blue", 0x0000ff),
    ("blueviolet", 0x8a2be2),
    ("brown", 0xa52a2a),
    ("burlywood", 0xdeb887),
    ("cadetblue", 0x5f9ea0),
    ("chartreuse", 0x7fff00),
    ("chocolate", 0xd2691e),
    ("coral", 0xff7f50),
    ("cornflowerblue", 0x6495ed),
    ("cornsilk", 0xfff8dc),
    ("crimson", 0xdc143c),
    ("cyan", 0x00ffff),
    ("darkblue", 0x00008b),
    ("darkcyan", 0x008b8b),
    ("darkgoldenrod", 0xb8860b),
    ("darkgray", 0xa9a9a9),
    ("darkgreen", 0x006400),
    ("darkgrey", 0xa9a9a9),
    ("darkkhaki", 0xbdb76b),
    ("darkmagenta", 0x8b008b),
    ("darkolivegreen", 0x556b2f),
    ("darkorange", 0xff8c00),
    ("darkorchid", 0x9932cc),
    ("darkred", 0x8b0000),
    ("darksalmon", 0xe9967a),
    ("darkseagreen", 0x8fbc8f),
    ("darkslateblue", 0x483d8b),
    ("darkslategray", 0x2f4f4f),
    ("darkslategrey", 0x2f4f4f),
    ("darkturquoise", 0x00ced1),
    ("darkviolet", 0x9400d3),
    ("deeppink", 0xff1493),
    ("deepskyblue", 0x00bfff),
    ("dimgray", 0x696969),
    ("dimgrey", 0x696969),
    ("dodgerblue", 0x1e90ff),
    ("firebrick", 0xb22222),
    ("floralwhite", 0xfffaf0),
    ("forestgreen", 0x228b22),
    ("fuchsia", 0xff00ff),
    ("gainsboro", 0xdcdcdc),
    ("ghostwhite", 0xf8f8ff),
    ("gold", 0xffd700),
    ("goldenrod", 0xdaa520),
    ("gray", 0x808080),
    ("green", 0x008000),
    ("greenyellow", 0xadff2f),
    ("grey", 0x808080),
    ("honeydew", 0xf0fff0),
    ("hotpink", 0xff69b4),
    ("indianred", 0xcd5c5c),
    ("indigo", 0x4b0082),
    ("ivory", 0xfffff0),
    ("khaki", 0xf0e68c),
    ("lavender", 0xe6e6fa),
    ("lavenderblush", 0xfff0f5),
    ("lawngreen", 0x7cfc00),
    ("lemonchiffon", 0xfffacd),
    ("lightblue", 0xadd8e6),
    ("lightcoral", 0xf08080),
    ("lightcyan", 0xe0ffff),
    ("lightgoldenrodyellow", 0xfafad2),
    ("lightgray", 0xd3d3d3),
    ("lightgreen", 0x90ee90),
    ("lightgrey", 0xd3d3d3),
    ("lightpink", 0xffb6c1),
    ("lightsalmon", 0xffa07a),
    ("lightseagreen", 0x20b2aa),
    ("lightskyblue", 0x87cefa),
    ("lightslategray", 0x778899),
    ("lightslategrey", 0x778899),
    ("lightsteelblue", 0xb0c4de),
    ("lightyellow", 0xffffe0),
    ("lime", 0x00ff00),
    ("limegreen", 0x32cd32),
    ("linen", 0xfaf0e6),
    ("magenta", 0xff00ff),
    ("maroon", 0x800000),
    ("mediumaquamarine", 0x66cdaa),
    ("mediumblue", 0x0000cd),
    ("mediumorchid", 0xba55d3),
    ("mediumpurple", 0x9370db),
    ("mediumseagreen", 0x3cb371),
    ("mediumslateblue", 0x7b68ee),
    ("mediumspringgreen", 0x00fa9a),
    ("mediumturquoise", 0x48d1cc),
    ("mediumvioletred", 0xc71585),
    ("midnightblue", 0x191970),
    ("mintcream", 0xf5fffa),
    ("mistyrose", 0xffe4e1),
    ("moccasin", 0xffe4b5),
    ("navajowhite", 0xffdead),
    ("navy", 0x000080),
    ("oldlace", 0xfdf5e6),
    ("olive", 0x808000),
    ("olivedrab", 0x6b8e23),
    ("orange", 0xffa500),
    ("orangered", 0xff4500),
    ("orchid", 0xda70d6),
    ("palegoldenrod", 0xeee8aa),
    ("palegreen", 0x98fb98),
    ("paleturquoise", 0xafeeee),
    ("palevioletred", 0xdb7093),
    ("papayawhip", 0xffefd5),
    ("peachpuff", 0xffdab9),
    ("peru", 0xcd853f),
    ("pink", 0xffc0cb),
    ("plum", 0xdda0dd),
    ("powderblue", 0xb0e0e6),
    ("purple", 0x800080),
    ("rebeccapurple", 0x663399),
    ("red", 0xff0000),
    ("rosybrown", 0xbc8f8f),
    ("royalblue", 0x4169e1),
    ("saddlebrown", 0x8b4513),
    ("salmon", 0xfa8072),
    ("sandybrown", 0xf4a460),
    ("seagreen", 0x2e8b57),
    ("seashell", 0xfff5ee),
    ("sienna", 0xa0522d),
    ("silver", 0xc0c0c0),
    ("skyblue", 0x87ceeb),
    ("slateblue", 0x6a5acd),
    ("slategray", 0x708090),
    ("slategrey", 0x708090),
    ("snow", 0xfffafa),
    ("springgreen", 0x00ff7f),
    ("steelblue", 0x4682b4),
    ("tan", 0xd2b48c),
    ("teal", 0x008080),
    ("thistle", 0xd8bfd8),
    ("tomato", 0xff6347),
    ("turquoise", 0x40e0d0),
    ("violet", 0xee82ee),
    ("wheat", 0xf5deb3),
    ("white", 0xffffff),
    ("whitesmoke", 0xf5f5f5),
    ("yellow", 0xffff00),
    ("yellowgreen", 0x9acd32),
];

#[cfg(test)]
pub(super) fn named_count() -> usize {
    NAMED.len()
}
