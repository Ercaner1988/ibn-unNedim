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
"#;

const DURUM: &str = r#"{"zaman":1,"parca_vektoru":[2,3],"ocr_kuyrugu":4,"gomme":{"calisiyor":true,"neden":null,"bugun":5},"ocr":{"calisiyor":false,"neden":"saat dışı","bugun":6}}"#;

#[test]
fn ornek_politika_okunur() {
    let p = Politika::ayristir(ORNEK).unwrap();
    assert_eq!((p.sinirlar.bellek_yuzde, p.sinirlar.cpu_yuzde), (90, 50));
    assert!(p.sinirlar.kullanirken_dur);
    assert_eq!(p.ocr.saatler, "00:00-06:30");
    assert_eq!(p.gomme.saatler, "her zaman");
}

#[test]
fn durum_json_alanlari() {
    let d: ArkaplanDurumu = serde_json::from_str(DURUM).unwrap();
    assert_eq!((d.zaman, d.parca_vektoru, d.ocr_kuyrugu), (1, (2, 3), 4));
    assert_eq!(d.ocr.neden.as_deref(), Some("saat dışı"));
    assert_eq!((d.gomme.bugun, d.ocr.bugun), (5, 6));
}
