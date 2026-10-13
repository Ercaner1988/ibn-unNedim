//! Arka plan işleri için ortak politika (ibnü'n-Nedim + Nazar): `arkaplan.toml` biçimi,
//! okuma/yazma, "şimdi çalışabilir mi?" kararı ve ölçümler. Tasarım:
//! docs/superpowers/specs/2026-10-12-arka-plan-sekmesi-design.md

mod dosya;
mod karar;
pub mod sistem;

pub use dosya::*;
pub use karar::*;

use serde::{Deserialize, Serialize};

pub const SURUM: u32 = 1;
/// `duraklat_bitis` için "ben sürdürene dek" (TOML tamsayısı i64'tür; u64::MAX yazılamaz).
pub const SURDURENE_DEK: i64 = i64::MAX;
pub const HER_ZAMAN: &str = "her zaman";
/// CPU sınırından düşülen arka plan süreçleri (küçük harf).
pub const ARKA_PLAN_SURECLERI: &[&str] = &["ibnun-nedim.exe", "nazard.exe", "tesseract.exe"];

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Politika {
    pub surum: u32,
    #[serde(default)]
    pub sinirlar: Sinirlar,
    #[serde(default)]
    pub gomme: IsAyari,
    #[serde(default)]
    pub ocr: IsAyari,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Sinirlar {
    pub bellek_yuzde: u32,
    pub cpu_yuzde: u32,
    pub kullanirken_dur: bool,
}

impl Default for Sinirlar {
    fn default() -> Self {
        Self {
            bellek_yuzde: 90,
            cpu_yuzde: 50,
            kullanirken_dur: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct IsAyari {
    pub saatler: String,
    pub duraklat_bitis: i64,
}

impl Default for IsAyari {
    fn default() -> Self {
        Self {
            saatler: HER_ZAMAN.into(),
            duraklat_bitis: 0,
        }
    }
}

/// Bugünkü Nazar davranışı: bellek %90, CPU %50, saat kısıtı yok, duraklatma yok.
impl Default for Politika {
    fn default() -> Self {
        Self {
            surum: SURUM,
            sinirlar: Sinirlar::default(),
            gomme: IsAyari::default(),
            ocr: IsAyari::default(),
        }
    }
}

impl Politika {
    pub fn is(&self, t: IsTuru) -> &IsAyari {
        match t {
            IsTuru::Gomme => &self.gomme,
            IsTuru::Ocr => &self.ocr,
        }
    }

    pub fn is_mut(&mut self, t: IsTuru) -> &mut IsAyari {
        match t {
            IsTuru::Gomme => &mut self.gomme,
            IsTuru::Ocr => &mut self.ocr,
        }
    }

    /// TOML'dan okur ve doğrular: bilinmeyen sürüm, geçersiz saat aralığı, 1-100 dışı yüzde hata.
    pub fn ayristir(metin: &str) -> Result<Self, String> {
        let p: Politika = toml::from_str(metin).map_err(|e| e.to_string())?;
        if p.surum != SURUM {
            return Err(format!("bilinmeyen surum {} (beklenen {SURUM})", p.surum));
        }
        for (ad, is) in [("gomme", &p.gomme), ("ocr", &p.ocr)] {
            SaatAraligi::ayristir(&is.saatler).map_err(|e| format!("[{ad}] saatler: {e}"))?;
        }
        let s = &p.sinirlar;
        if !(1..=100).contains(&s.bellek_yuzde) || !(1..=100).contains(&s.cpu_yuzde) {
            return Err("sinirlar: yüzde 1-100 dışında".into());
        }
        Ok(p)
    }

    pub fn metin(&self) -> String {
        toml::to_string(self).expect("Politika her zaman TOML'a yazılır")
    }
}
