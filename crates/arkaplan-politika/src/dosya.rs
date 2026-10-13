//! Dosyalar: politika (`%APPDATA%\ibnun-nedim\arkaplan.toml`, ibnü'n-Nedim sahibi) ve Nazar'ın
//! durum dosyası (`%LOCALAPPDATA%\nazar\arkaplan-durum.json`). Yazma geçici dosya + ad değiştirme:
//! okuyan yarım dosya görmez.

use crate::Politika;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::SystemTime;

fn taban(degisken: &str) -> PathBuf {
    std::env::var_os(degisken)
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
}

pub fn politika_yolu() -> PathBuf {
    taban("APPDATA").join("ibnun-nedim").join("arkaplan.toml")
}

pub fn durum_yolu() -> PathBuf {
    taban("LOCALAPPDATA")
        .join("nazar")
        .join("arkaplan-durum.json")
}

fn atomik_yaz(yol: &Path, icerik: &[u8]) -> io::Result<()> {
    if let Some(d) = yol.parent() {
        fs::create_dir_all(d)?;
    }
    let gecici = yol.with_extension(format!("yaziliyor-{}", std::process::id()));
    fs::write(&gecici, icerik)?;
    fs::rename(&gecici, yol)
}

pub fn yaz(yol: &Path, p: &Politika) -> io::Result<()> {
    atomik_yaz(yol, p.metin().as_bytes())
}

#[derive(Clone, Debug, PartialEq)]
pub enum Okuma {
    Yok,
    Gecerli(Politika),
    /// Okunamadı ya da doğrulanmadı; son geçerli politika (varsa) sürer, dosyanın üstüne yazılmaz.
    Bozuk {
        hata: String,
        son_gecerli: Option<Politika>,
    },
}

/// Politikayı değişme tarihine bakarak yeniden okur (her denetimde maliyet yalnız `metadata`).
pub struct Okuyucu {
    yol: PathBuf,
    son: Mutex<(Option<SystemTime>, Okuma)>,
}

impl Okuyucu {
    pub fn yeni(yol: PathBuf) -> Self {
        Self {
            yol,
            son: Mutex::new((None, Okuma::Yok)),
        }
    }

    pub fn yol(&self) -> &Path {
        &self.yol
    }

    pub fn oku(&self) -> Okuma {
        let mut son = self.son.lock().unwrap_or_else(|e| e.into_inner());
        let Some(tarih) = fs::metadata(&self.yol).and_then(|m| m.modified()).ok() else {
            *son = (None, Okuma::Yok);
            return Okuma::Yok;
        };
        if son.0 == Some(tarih) {
            return son.1.clone();
        }
        let onceki = match &son.1 {
            Okuma::Gecerli(p) => Some(p.clone()),
            Okuma::Bozuk { son_gecerli, .. } => son_gecerli.clone(),
            Okuma::Yok => None,
        };
        let yeni = match fs::read_to_string(&self.yol) {
            Ok(m) => match Politika::ayristir(&m) {
                Ok(p) => Okuma::Gecerli(p),
                Err(hata) => Okuma::Bozuk {
                    hata,
                    son_gecerli: onceki,
                },
            },
            Err(e) => Okuma::Bozuk {
                hata: e.to_string(),
                son_gecerli: onceki,
            },
        };
        if let Okuma::Bozuk { hata, .. } = &yeni {
            eprintln!(
                "{} okunamadı, son geçerli politika sürüyor: {hata}",
                self.yol.display()
            );
        }
        *son = (Some(tarih), yeni.clone());
        yeni
    }

    /// Uygulanacak politika: geçerli, bozuksa son geçerli, dosya yoksa `None`.
    pub fn etkin(&self) -> Option<Politika> {
        match self.oku() {
            Okuma::Gecerli(p) => Some(p),
            Okuma::Bozuk { son_gecerli, .. } => son_gecerli,
            Okuma::Yok => None,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct IsDurumu {
    pub calisiyor: bool,
    pub neden: Option<String>,
    /// Bugün işlenen: gömmede parça, OCR'da belge.
    pub bugun: u64,
}

/// nazard'ın ~10 sn'de bir yazdığı durum; Arka plan sekmesi okur.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ArkaplanDurumu {
    pub zaman: i64,
    /// (vektörü olan, toplam) parça.
    pub parca_vektoru: (usize, usize),
    pub ocr_kuyrugu: usize,
    pub gomme: IsDurumu,
    pub ocr: IsDurumu,
}

pub fn durum_yaz(yol: &Path, d: &ArkaplanDurumu) -> io::Result<()> {
    atomik_yaz(
        yol,
        &serde_json::to_vec_pretty(d).map_err(io::Error::other)?,
    )
}

pub fn durum_oku(yol: &Path) -> Option<ArkaplanDurumu> {
    serde_json::from_slice(&fs::read(yol).ok()?).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn gecici(ad: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("arkaplan-politika-{ad}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d.join("arkaplan.toml")
    }

    /// Ardışık yazışlar aynı değişme tarihine düşmesin (okuyucu tarihe bakar).
    fn yaz_ileri(yol: &Path, metin: &str, ileri_sn: u64) {
        fs::write(yol, metin).unwrap();
        let f = fs::OpenOptions::new().write(true).open(yol).unwrap();
        f.set_modified(SystemTime::now() + Duration::from_secs(ileri_sn))
            .unwrap();
    }

    #[test]
    fn yazilan_okunur_ve_atomik() {
        let yol = gecici("yaz");
        let mut p = Politika::default();
        p.ocr.saatler = "00:00-06:30".into();
        yaz(&yol, &p).unwrap();
        assert_eq!(Okuyucu::yeni(yol.clone()).oku(), Okuma::Gecerli(p));
        let artik: Vec<_> = fs::read_dir(yol.parent().unwrap())
            .unwrap()
            .flatten()
            .filter(|g| g.file_name().to_string_lossy().contains("yaziliyor"))
            .collect();
        assert!(artik.is_empty(), "geçici dosya kalmaz");
    }

    #[test]
    fn dosya_yoksa_yok() {
        let o = Okuyucu::yeni(gecici("yok"));
        assert_eq!(o.oku(), Okuma::Yok);
        assert_eq!(o.etkin(), None);
    }

    #[test]
    fn bozuk_dosyada_son_gecerli_kalir() {
        let yol = gecici("bozuk");
        let o = Okuyucu::yeni(yol.clone());
        let p = Politika::default();
        yaz_ileri(&yol, &p.metin(), 1);
        assert_eq!(o.oku(), Okuma::Gecerli(p.clone()));
        yaz_ileri(&yol, "surum = 1\n[ocr]\nsaatler = \"gece\"\n", 2);
        match o.oku() {
            Okuma::Bozuk { hata, son_gecerli } => {
                assert!(hata.contains("[ocr]"), "{hata}");
                assert_eq!(son_gecerli, Some(p.clone()));
            }
            d => panic!("bozuk beklenirdi: {d:?}"),
        }
        assert_eq!(o.etkin(), Some(p), "son geçerli politika sürer");
        assert!(
            fs::read_to_string(&yol).unwrap().contains("gece"),
            "dosyanın üstüne yazılmaz"
        );
    }

    #[test]
    fn durum_gidis_donus() {
        let yol = gecici("durum").with_file_name("arkaplan-durum.json");
        assert_eq!(durum_oku(&yol), None);
        let d = ArkaplanDurumu {
            zaman: 5,
            parca_vektoru: (1, 2),
            ocr_kuyrugu: 3,
            gomme: IsDurumu {
                calisiyor: true,
                neden: None,
                bugun: 7,
            },
            ocr: IsDurumu {
                calisiyor: false,
                neden: Some("saat dışı".into()),
                bugun: 0,
            },
        };
        durum_yaz(&yol, &d).unwrap();
        assert_eq!(durum_oku(&yol), Some(d));
    }
}
