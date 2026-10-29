//! İki repo arası sözleşme: bu metinler Nazar'ın `crates/nazar-arkaplan/tests/sozlesme.rs`'inde
//! de AYNEN durur. Biri değişirse öteki de değişmeli.
use arkaplan_politika::{ArkaplanDurumu, Politika};

const ORNEK: &str = r#"surum = 1
[sinirlar]
bellek_yuzde = 90
cpu_yuzde = 50
kullanirken_dur = true
[gomme]
saatler = "her zaman"
duraklat_bitis = 0
[ocr]
saatler = "00:00-06:30"
duraklat_bitis = 0
[gorsel]
saatler = "22:00-06:30"
duraklat_bitis = 0
"#;

const DURUM: &str = r#"{"zaman":1,"parca_vektoru":[2,3],"ocr_kuyrugu":4,"gomme":{"calisiyor":true,"neden":null,"bugun":5},"ocr":{"calisiyor":false,"neden":"saat dışı","bugun":6},"gorsel":{"calisiyor":true,"neden":null,"bugun":2}}"#;

#[test]
fn ornek_politika_okunur() {
    let p = Politika::ayristir(ORNEK).unwrap();
    assert_eq!((p.sinirlar.bellek_yuzde, p.sinirlar.cpu_yuzde), (90, 50));
    assert!(p.sinirlar.kullanirken_dur);
    assert_eq!(p.ocr.saatler, "00:00-06:30");
    assert_eq!(p.gomme.saatler, "her zaman");
    assert_eq!(p.gorsel.saatler, "22:00-06:30", "görüntü gömme ayrı saat");
}

#[test]
fn gorsel_bolumu_olmayan_eski_dosya_okunur() {
    // 29.10.2026 öncesi yazılmış `arkaplan.toml`da `[gorsel]` yok: varsayılanla açılır.
    let eski = ORNEK.replace(
        "[gorsel]\nsaatler = \"22:00-06:30\"\nduraklat_bitis = 0\n",
        "",
    );
    assert!(!eski.contains("[gorsel]"));
    let p = Politika::ayristir(&eski).unwrap();
    assert_eq!(p.gorsel.saatler, arkaplan_politika::HER_ZAMAN);
    assert_eq!(p.gorsel.duraklat_bitis, 0);
}

#[test]
fn durum_json_alanlari() {
    let d: ArkaplanDurumu = serde_json::from_str(DURUM).unwrap();
    assert_eq!((d.zaman, d.parca_vektoru, d.ocr_kuyrugu), (1, (2, 3), 4));
    assert_eq!(d.ocr.neden.as_deref(), Some("saat dışı"));
    assert_eq!((d.gomme.bugun, d.ocr.bugun), (5, 6));
    assert!(d.gorsel.calisiyor, "görüntü işi çalışıyor görünmeli");
    assert_eq!(d.gorsel.bugun, 2);
}

#[test]
fn eski_durum_json_gorselsiz_okunur() {
    // 29.10.2026 öncesi yazılmış durum dosyasında `gorsel` yok: `Default` ile açılır.
    let eski = DURUM.replace(
        ",\"gorsel\":{\"calisiyor\":true,\"neden\":null,\"bugun\":2}",
        "",
    );
    assert!(!eski.contains("gorsel"));
    let d: ArkaplanDurumu = serde_json::from_str(&eski).unwrap();
    assert_eq!(d.gorsel, arkaplan_politika::IsDurumu::default());
    assert_eq!(d.gomme.bugun, 5);
}
