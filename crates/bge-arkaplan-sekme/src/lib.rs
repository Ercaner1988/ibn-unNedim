//! Arka plan sekmesi: toplu gömme (bu sunucu) ve OCR (Nazar) için durum, elle duraklatma, saat
//! aralığı ve ortak sınırlar. Politika `arkaplan.toml`'a anında yazılır (arkaplan-politika).
//! Dosya işi çizim iş parçacığında yapılmaz: bir işçi okur/yazar, sonucu kanaldan gönderir.

mod mantik;
pub use mantik::*;

use arkaplan_politika::{
    self as ap, ArkaplanDurumu, Duraklatma, IsTuru, Okuma, Politika, SaatAraligi,
};
use bge_settings::Status;
use bge_theme::{self as theme, Palette, Weight};
use eframe::egui::{self, Ui};
use std::sync::atomic::Ordering;
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::time::Duration;

/// İşçinin gönderdiği anlık durum: politika okuması, Nazar durumu, son yazma hatası.
#[derive(Clone, Debug, PartialEq)]
struct Anlik {
    okuma: Okuma,
    nazar: Option<ArkaplanDurumu>,
    yazma_hatasi: Option<String>,
    /// Bugün gömülen metin; işçi 5 sn'de bir sayar (sekme hiç açılmasa da gün dönümü kaçmaz).
    bugun: u64,
}

/// Anahtar sütununun genişliği (`alan_satiri`).
const ANAHTAR: f32 = 96.0;
const SAAT_KUTUSU: f32 = 56.0;

pub struct Sekme {
    rx: Receiver<Anlik>,
    tx: Sender<Politika>,
    anlik: Option<Anlik>,
    /// Saat kutuları [gömme, ocr] × (baş, son); işçiden yeni anlık gelince doldurulur.
    saat: [(String, String); 2],
    saat_hatasi: [bool; 2],
    menu: Option<IsTuru>,
}

impl Sekme {
    /// Dosya işini yapan işçiyi açar: 5 sn'de bir (ya da yazma isteği gelince) okur, değişiklik
    /// varsa gönderir ve yeniden çizim ister. Çizim iş parçacığı dosyaya dokunmaz.
    pub fn yeni(ctx: &egui::Context, status: std::sync::Arc<Status>) -> Self {
        let (atx, arx) = channel::<Anlik>();
        let (ptx, prx) = channel::<Politika>();
        let ctx = ctx.clone();
        std::thread::Builder::new()
            .name("arkaplan-sekme".into())
            .spawn(move || {
                let okuyucu = ap::Okuyucu::yeni(ap::politika_yolu());
                let durum = ap::durum_yolu();
                let mut onceki: Option<Anlik> = None;
                let mut yazma_hatasi = None;
                // (tarih, gün başındaki `texts_embedded`); sayaç süreç açılınca 0'dan başlar.
                let mut taban = (ap::sistem::yerel_tarih(), 0);
                loop {
                    let a = Anlik {
                        okuma: okuyucu.oku(),
                        nazar: ap::durum_oku(&durum),
                        yazma_hatasi: yazma_hatasi.take(),
                        bugun: bugun_farki(
                            &mut taban,
                            ap::sistem::yerel_tarih(),
                            status.texts_embedded.load(Ordering::Relaxed),
                        ),
                    };
                    if onceki.as_ref() != Some(&a) {
                        if atx.send(a.clone()).is_err() {
                            return;
                        }
                        ctx.request_repaint();
                        onceki = Some(a);
                    }
                    match prx.recv_timeout(Duration::from_secs(5)) {
                        Ok(mut p) => {
                            // Sürükleme sırasında biriken istekler: yalnız sonuncusu yazılır.
                            while let Ok(q) = prx.try_recv() {
                                p = q;
                            }
                            if let Err(e) = ap::yaz(okuyucu.yol(), &p) {
                                yazma_hatasi = Some(e.to_string());
                            }
                        }
                        Err(RecvTimeoutError::Timeout) => {}
                        Err(RecvTimeoutError::Disconnected) => return,
                    }
                }
            })
            .expect("arkaplan-sekme iş parçacığı açılamadı");
        Self {
            rx: arx,
            tx: ptx,
            anlik: None,
            saat: Default::default(),
            saat_hatasi: [false; 2],
            menu: None,
        }
    }

    /// Ekranda kullanılan politika: geçerli, bozuksa son geçerli, yoksa varsayılan.
    fn politika(&self) -> Politika {
        match self.anlik.as_ref().map(|a| &a.okuma) {
            Some(Okuma::Gecerli(p))
            | Some(Okuma::Bozuk {
                son_gecerli: Some(p),
                ..
            }) => p.clone(),
            _ => Politika::default(),
        }
    }

    /// Değişikliği işçiye yazdırır; işçi yeniden okuyana dek ekranda yeni değer görünür.
    fn degistir(&mut self, f: impl FnOnce(&mut Politika)) {
        let mut p = self.politika();
        f(&mut p);
        if let Some(a) = self.anlik.as_mut() {
            a.okuma = Okuma::Gecerli(p.clone());
        }
        let _ = self.tx.send(p);
    }

    fn saatleri_doldur(&mut self) {
        let p = self.politika();
        for is in [IsTuru::Gomme, IsTuru::Ocr] {
            let a = match SaatAraligi::ayristir(&p.is(is).saatler) {
                Ok(SaatAraligi::Aralik { bas, son }) => (bas, son),
                _ => match VARSAYILAN_ARALIK {
                    SaatAraligi::Aralik { bas, son } => (bas, son),
                    SaatAraligi::HerZaman => (0, 390),
                },
            };
            let m = |d: u32| format!("{:02}:{:02}", d / 60, d % 60);
            let Some(i) = sira(is) else { continue };
            self.saat[i] = (m(a.0), m(a.1));
        }
    }

    fn taze_nazar(&self, simdi: i64) -> Option<ArkaplanDurumu> {
        self.anlik
            .as_ref()?
            .nazar
            .clone()
            .filter(|d| simdi - d.zaman <= BAYAT_SN)
    }

    pub fn ui(&mut self, ui: &mut Ui, p: Palette) {
        let onceki = self.anlik.as_ref().map(|a| a.okuma.clone());
        while let Ok(a) = self.rx.try_recv() {
            self.anlik = Some(a);
        }
        // Yalnız politika değişince: Nazar durumu ~10 sn'de bir gelir, yazılmakta olan saati ezerdi.
        if self.anlik.as_ref().map(|a| &a.okuma) != onceki.as_ref() {
            self.saatleri_doldur();
        }
        let simdi = ap::sistem::simdi();
        let gun_sn = ap::sistem::gun_saniyesi();
        self.render_dosya_notu(ui, p);
        self.render_gomme(ui, p, simdi, gun_sn);
        ui.add_space(theme::GAP_MD);
        self.render_ocr(ui, p, simdi, gun_sn);
        ui.add_space(theme::GAP_MD);
        self.render_sinirlar(ui, p);
    }

    fn render_dosya_notu(&self, ui: &mut Ui, p: Palette) {
        let Some(a) = &self.anlik else { return };
        let (metin, renk) = match (&a.yazma_hatasi, &a.okuma) {
            (Some(e), _) => (format!("Kaydedilemedi: {e}"), p.error),
            (None, Okuma::Bozuk { hata, .. }) => (
                format!(
                    "arkaplan.toml okunamadı ({hata}); son geçerli ayar sürüyor. İlk değişiklik dosyayı düzeltir."
                ),
                p.error,
            ),
            (None, Okuma::Yok) => (
                "Ayar dosyası yok: varsayılanlar gösteriliyor, toplu gömme kısıtlanmıyor. İlk değişiklik dosyayı oluşturur."
                    .into(),
                p.text_muted,
            ),
            (None, Okuma::Gecerli(_)) => return,
        };
        ui.label(theme::rich(
            metin,
            theme::SIZE_CAPTION,
            Weight::Regular,
            renk,
        ));
        ui.add_space(theme::GAP_SM);
    }

    fn render_gomme(&mut self, ui: &mut Ui, p: Palette, simdi: i64, gun_sn: Option<u32>) {
        let pol = self.politika();
        let isler = bge_clients::defter().isler();
        let bekleyen = isler
            .iter()
            .any(|j| j.evre == bge_clients::Evre::PolitikaBekliyor);
        let hesaplanan = isler
            .iter()
            .any(|j| j.evre == bge_clients::Evre::TopluHesap);
        let durum = gomme_durumu(
            &pol,
            simdi,
            gun_sn,
            bge_clients::politika::KAPI.son_neden(),
            bekleyen,
            hesaplanan,
        );
        let bugun = self.anlik.as_ref().map_or(0, |a| a.bugun);
        let mut sayilar = format!("bugün {bugun} metin");
        if let Some(d) = self.taze_nazar(simdi) {
            sayilar += &format!(
                " · Nazar parça vektörü {}/{}",
                d.parca_vektoru.0, d.parca_vektoru.1
            );
        }
        self.render_is(
            ui,
            p,
            IsTuru::Gomme,
            "Toplu gömme",
            &pol,
            durum,
            sayilar,
            (simdi, gun_sn),
        );
    }

    fn render_ocr(&mut self, ui: &mut Ui, p: Palette, simdi: i64, gun_sn: Option<u32>) {
        let pol = self.politika();
        let nazar = self.taze_nazar(simdi);
        let durum = ocr_durumu(&pol, simdi, gun_sn, nazar.as_ref());
        let sayilar = match &nazar {
            Some(d) => format!("kuyruk {} · bugün {} belge", d.ocr_kuyrugu, d.ocr.bugun),
            None => "nazard'dan durum gelmiyor".into(),
        };
        self.render_is(
            ui,
            p,
            IsTuru::Ocr,
            "OCR (Nazar)",
            &pol,
            durum,
            sayilar,
            (simdi, gun_sn),
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn render_is(
        &mut self,
        ui: &mut Ui,
        p: Palette,
        is: IsTuru,
        baslik: &str,
        pol: &Politika,
        durum: SatirDurumu,
        sayilar: String,
        (simdi, gun_sn): (i64, Option<u32>),
    ) {
        let renk = match durum.tur {
            Tur::Calisiyor => p.success,
            Tur::Bekliyor | Tur::Duraklatildi => p.warning,
            Tur::Bosta => p.text_muted,
            Tur::Hata => p.error,
        };
        theme::card(ui, p, |ui| {
            ui.label(theme::rich(
                baslik,
                theme::SIZE_BODY,
                Weight::SemiBold,
                p.text,
            ));
            ui.add_space(theme::GAP_XS);
            kilim_tema::alan_satiri(ui, ANAHTAR, "Durum", |ui| {
                theme::status_dot_row(ui, p, renk, &durum.metin)
            });
            kilim_tema::alan_satiri(ui, ANAHTAR, "Sayılar", |ui| {
                ui.label(theme::rich(
                    sayilar,
                    theme::SIZE_CAPTION,
                    Weight::Regular,
                    p.text_muted,
                ))
            });
            kilim_tema::alan_satiri(ui, ANAHTAR, "Saatler", |ui| {
                self.render_saat(ui, p, is, pol)
            });
            ui.add_space(theme::GAP_SM);
            self.render_duraklat(ui, p, is, pol, simdi, gun_sn);
        });
    }

    fn render_saat(&mut self, ui: &mut Ui, p: Palette, is: IsTuru, pol: &Politika) {
        let Some(i) = sira(is) else { return };
        let mut her_zaman = matches!(
            SaatAraligi::ayristir(&pol.is(is).saatler),
            Ok(SaatAraligi::HerZaman) | Err(_)
        );
        ui.horizontal_wrapped(|ui| {
            if theme::toggle(ui, p, &mut her_zaman).changed() {
                let a = if her_zaman {
                    SaatAraligi::HerZaman
                } else {
                    SaatAraligi::ayristir(&format!("{}-{}", self.saat[i].0, self.saat[i].1))
                        .unwrap_or(VARSAYILAN_ARALIK)
                };
                self.degistir(|q| saat_ayarla(q, is, a));
            }
            ui.label(theme::rich(
                "her zaman",
                theme::SIZE_CAPTION,
                Weight::Regular,
                p.text,
            ));
            if her_zaman {
                return;
            }
            let (r1, r2) = saat_araligi(ui, p, &mut self.saat[i]);
            if r1.lost_focus() || r2.lost_focus() {
                match SaatAraligi::ayristir(&format!("{}-{}", self.saat[i].0, self.saat[i].1)) {
                    Ok(a) => {
                        self.saat_hatasi[i] = false;
                        self.degistir(|q| saat_ayarla(q, is, a));
                    }
                    Err(_) => self.saat_hatasi[i] = true,
                }
            }
            if self.saat_hatasi[i] {
                ui.label(theme::rich(
                    "SS:DD biçiminde, başlangıç ≠ bitiş",
                    theme::SIZE_CAPTION,
                    Weight::Regular,
                    p.error,
                ));
            }
        });
    }

    fn render_duraklat(
        &mut self,
        ui: &mut Ui,
        p: Palette,
        is: IsTuru,
        pol: &Politika,
        simdi: i64,
        gun_sn: Option<u32>,
    ) {
        let durdu = duraklatildi_mi(pol, is, simdi);
        ui.horizontal_wrapped(|ui| {
            if durdu {
                if theme::secondary_button(ui, p, "Sürdür", true).clicked() {
                    self.degistir(|q| surdur(q, is));
                }
            } else if theme::secondary_button(ui, p, "Duraklat…", true).clicked() {
                self.menu = if self.menu == Some(is) {
                    None
                } else {
                    Some(is)
                };
            }
        });
        if self.menu == Some(is) && !durdu {
            ui.horizontal_wrapped(|ui| {
                for (ad, d) in [
                    ("1 saat", Duraklatma::BirSaat),
                    ("bugün boyunca", Duraklatma::BugunBoyunca),
                    ("ben sürdürene dek", Duraklatma::SurdureneDek),
                ] {
                    if theme::secondary_button(ui, p, ad, true).clicked() {
                        self.degistir(|q| duraklat(q, is, d, simdi, gun_sn));
                        self.menu = None;
                    }
                }
            });
        }
    }

    fn render_sinirlar(&mut self, ui: &mut Ui, p: Palette) {
        let pol = self.politika();
        let s = &pol.sinirlar;
        let (mut bellek, mut cpu, mut kullanirken) =
            (s.bellek_yuzde, s.cpu_yuzde, s.kullanirken_dur);
        let mut degisti = false;
        theme::card(ui, p, |ui| {
            ui.label(theme::rich(
                "Ortak sınırlar",
                theme::SIZE_BODY,
                Weight::SemiBold,
                p.text,
            ));
            ui.add_space(theme::GAP_XS);
            kilim_tema::alan_satiri(ui, ANAHTAR, "Bellek", |ui| {
                degisti |= ui
                    .add(
                        egui::DragValue::new(&mut bellek)
                            .range(50..=99)
                            // Elle yazılmış aralık dışı değer (ör. 100 = sınır yok) kırpılıp dosyaya yazılmasın.
                            .clamp_existing_to_range(false)
                            .suffix(" %"),
                    )
                    .changed();
            });
            kilim_tema::alan_satiri(ui, ANAHTAR, "İşlemci", |ui| {
                degisti |= ui
                    .add(
                        egui::DragValue::new(&mut cpu)
                            .range(10..=95)
                            .clamp_existing_to_range(false)
                            .suffix(" %"),
                    )
                    .changed();
            });
            kilim_tema::alan_satiri(ui, ANAHTAR, "Kullanırken", |ui| {
                ui.horizontal(|ui| {
                    degisti |= theme::toggle(ui, p, &mut kullanirken).changed();
                    ui.label(theme::rich(
                        "bilgisayarı kullanırken dur",
                        theme::SIZE_CAPTION,
                        Weight::Regular,
                        p.text,
                    ));
                });
            });
            ui.label(theme::rich(
                "İşlemci sınırı yalnız senin programlarının yüküne bakar; ibnü'n-Nedim, nazard ve tesseract düşülür.",
                theme::SIZE_CAPTION,
                Weight::Regular,
                p.text_muted,
            ));
        });
        if degisti {
            self.degistir(|q| {
                q.sinirlar.bellek_yuzde = bellek;
                q.sinirlar.cpu_yuzde = cpu;
                q.sinirlar.kullanirken_dur = kullanirken;
            });
        }
    }
}

/// Saat aralığı kutuları (başlangıç – bitiş): sarmalı satırda TEK birim; ayrı birimler "00:00 –" üstte,
/// bitiş altta kalacak biçimde bölünüyordu.
fn saat_araligi(
    ui: &mut Ui,
    p: Palette,
    saat: &mut (String, String),
) -> (egui::Response, egui::Response) {
    let boyut = egui::vec2(SAAT_KUTUSU, theme::INPUT_HEIGHT);
    // İki kutu + tire + iki aralık.
    kilim_tema::birim(ui, 2.0 * SAAT_KUTUSU + 24.0, |ui| {
        ui.horizontal(|ui| {
            let r1 = theme::text_input(ui, p, &mut saat.0, boyut);
            ui.label(theme::rich(
                "–",
                theme::SIZE_CAPTION,
                Weight::Regular,
                p.text,
            ));
            (r1, theme::text_input(ui, p, &mut saat.1, boyut))
        })
        .inner
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// İşçisiz sekme: anlıkları sınama gönderir, yazma isteklerini sınama okur.
    fn isciz() -> (Sekme, Sender<Anlik>, Receiver<Politika>) {
        let (atx, arx) = channel();
        let (ptx, prx) = channel();
        let s = Sekme {
            rx: arx,
            tx: ptx,
            anlik: None,
            saat: Default::default(),
            saat_hatasi: [false; 2],
            menu: None,
        };
        (s, atx, prx)
    }

    fn anlik(p: &Politika, zaman: i64) -> Anlik {
        Anlik {
            okuma: Okuma::Gecerli(p.clone()),
            nazar: Some(ArkaplanDurumu {
                zaman,
                ..Default::default()
            }),
            yazma_hatasi: None,
            bugun: 0,
        }
    }

    /// Sekmeyi bir kare çizer.
    fn ciz(ctx: &egui::Context, s: &mut Sekme) {
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            let p = theme::current(ui.ctx());
            s.ui(ui, p);
        });
        out.textures_delta.clear(); // çizici yok
    }

    #[test]
    fn yazilan_saat_nazar_durumuyla_silinmez() {
        // nazard durum dosyasını ~10 sn'de bir yazar; her yeni anlık saat kutularını dosyadaki
        // değerle yeniden dolduruyor, kullanıcının yazmakta olduğu "06:3"ü siliyordu.
        let ctx = egui::Context::default();
        theme::apply(&ctx);
        let (mut s, atx, _prx) = isciz();
        let mut p = Politika::default();
        p.ocr.saatler = "00:00-06:30".into();
        atx.send(anlik(&p, 1)).unwrap();
        ciz(&ctx, &mut s);
        assert_eq!(s.saat[1].1, "06:30");
        s.saat[1].1 = "06:3".into();
        atx.send(anlik(&p, 2)).unwrap(); // yalnız Nazar durumu değişti
        ciz(&ctx, &mut s);
        assert_eq!(s.saat[1].1, "06:3", "yazılan saat silindi");
    }

    #[test]
    fn aralik_disi_sinir_sekme_acilinca_yazilmaz() {
        // Elle "cpu_yuzde = 100" (sınır yok) yazılmışsa DragValue onu 95'e kırpıp değişti
        // sayıyor, sekme açılır açılmaz dosya sessizce yeniden yazılıyordu.
        let ctx = egui::Context::default();
        theme::apply(&ctx);
        let (mut s, atx, prx) = isciz();
        let mut p = Politika::default();
        p.sinirlar.cpu_yuzde = 100;
        p.sinirlar.bellek_yuzde = 100;
        atx.send(anlik(&p, 1)).unwrap();
        for _ in 0..3 {
            ciz(&ctx, &mut s);
        }
        assert!(prx.try_recv().is_err(), "dokunulmadan yazma istendi");
    }

    #[test]
    fn saat_araligi_tek_parca_sarilir() {
        // OCR satırı: anahtar + "her zaman" + aralık. Dar satırda "00:00 –" üstte, "06:30" altta
        // kalıyordu; aralık bütün olarak alt satıra inmeli.
        let ctx = egui::Context::default();
        theme::apply(&ctx);
        let mut saat = ("00:00".to_string(), "06:30".to_string());
        let mut ust = (0.0, 1.0);
        for _ in 0..3 {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(260.0, 300.0),
                )),
                ..Default::default()
            };
            let mut out = ctx.run_ui(input, |ui| {
                let p = theme::current(ui.ctx());
                ui.horizontal_wrapped(|ui| {
                    ui.label("anahtar ve her zaman metni");
                    let (a, b) = saat_araligi(ui, p, &mut saat);
                    ust = (a.rect.top(), b.rect.top());
                });
            });
            out.textures_delta.clear(); // çizici yok
        }
        assert_eq!(ust.0, ust.1, "aralık bölündü");
    }
}
