# Arka plan sekmesi: toplu gömme ve OCR'ı tek yerden yönetmek

Tarih: 2026-10-12 · Durum: tasarım, inceleme bekliyor · Repolar: ibnun-nedim (bu repo), Nazar

## Amaç

ibnü'n-Nedim'in toplu gömme işleri ve Nazar'ın (nazard) kuyruk OCR'ı arka planda bellek ve
işlemci tüketiyor. Kullanıcı bunları bugün yalnız ayar dosyasıyla (`ocr_saatleri`) ya da
süreci kapatarak denetleyebiliyor. Bu tasarım ibnü'n-Nedim penceresine sade bir **Arka plan**
sekmesi ekler. Sekmeden şunlar yapılır:

- işlerin o anki durumu ve nedeni görülür,
- işler elle duraklatılır (süre seçmeli),
- belli saatlere kısıtlanır,
- bellek ve işlemci sınırına ya da kullanıcının bilgisayarı kullanmasına göre kendiliğinden durdurulur.

Seçimler (beyin fırtınası, 2026-10-12):

- İki iş, toplu gömme ve OCR, tek sekmede gösterilir.
- Her iş türüne bir satır ayrılır.
- Duraklatma süre seçmelidir.
- Yaklaşım A uygulanır: ortak politika dosyası ve ortak karar sandığı.

## 1. Sözleşme

### 1.1 Politika dosyası

Dosya: `%APPDATA%\ibnun-nedim\arkaplan.toml`. Sahibi ibnü'n-Nedim'dir; arayüz yazar, iki süreç
okur. Elle de düzenlenebilir.

```toml
surum = 1
[sinirlar]                 # bütün arka plan işleri için ortak
bellek_yuzde = 90          # sistem belleği bunun üstündeyse durur
cpu_yuzde = 50             # KULLANICININ işleri bunun üstündeyse durur (1.4)
kullanirken_dur = true     # son 60 sn'de klavye/fare varsa durur
[gomme]                    # ibnü'n-Nedim'in toplu işleri, bütün araçlar
saatler = "her zaman"      # ya da "00:00-06:30"
duraklat_bitis = 0         # unix sn; 0 = çalışıyor, u64::MAX = "ben sürdürene dek"
[ocr]                      # Nazar'ın kuyruk OCR'ı
saatler = "00:00-06:30"
duraklat_bitis = 0
```

- **Yeniden başlatma:** Duraklatmanın bitiş zamanı dosyada yazılıdır. Bu yüzden "1 saat" ya da
  "bugün boyunca" seçilen duraklatma süreç yeniden başlasa da sürer. Süre dolunca kendiliğinden kalkar.
- **Saat aralığı gece yarısını aşabilir:** örneğin `"22:00-06:30"`.

### 1.2 Durum dosyası

Dosya: `%LOCALAPPDATA%\nazar\arkaplan-durum.json`. nazard bu dosyayı yaklaşık 10 saniyede bir
yazar. Dosyada şunlar bulunur:

- parça vektörü kapsamı,
- OCR kuyruğu,
- bugün işlenen belge ve parça sayıları,
- her iş için o anki durum ve nedeni ("saat dışı", "bellek %93", "kullanıcı etkin", "elle duraklatıldı").

Sekme bu dosyayı okur. ibnü'n-Nedim kendi toplu iş kuyruğunu zaten bellekte tuttuğu için onun
tarafında ayrı bir dosya gerekmez. Bu dosya Nazar'ın dışa açık durum biçimidir; ikili depolama
kuralının (rkyv) kapsamına girmez, JSON'dur.

### 1.3 Ortak sandık `arkaplan-politika`

Bu repoda yeni, küçük bir sandık açılır. İçeriği:

- dosya biçimi,
- okuma ve yazma,
- "bu iş şimdi çalışabilir mi?" kararı, gerekçesiyle birlikte,
- saat aralığı denetimi,
- yerel saat ölçümü,
- kullanıcı etkinliği (Windows `GetLastInputInfo`),
- arka plan süreçlerini hariç tutan CPU ölçümü.

Nazar bu sandığa sabit bir git rev ile bağlanır, kilim-tema'ya bağlandığı gibi. Böylece biçim ve
karar mantığı tek yerde yazılır ve iki taraf birbirinden sapmaz. Bu repo ağırlık dosyası içermediği
için bağımlılık hafiftir.

### 1.4 Neden "kullanıcının işleri"?

CPU sınırı sistemin toplam yüküne bakarsa salınım olur:

1. ibnü'n-Nedim kendi gömmesiyle CPU'yu %50'nin üstüne çıkarır.
2. Bu yüzden kendini durdurur.
3. İş durunca yük düşer ve yeniden başlar; döngü böyle sürer.

Bunu önlemek için ölçüm, toplam yükten arka plan süreçlerinin payını düşer (ibnun-nedim, nazard,
tesseract). Sınır böylece yalnız kullanıcının çalıştırdığı programların yüküne uygulanır.

## 2. ibnü'n-Nedim tarafı

### 2.1 Toplu iş kapısı

- **Sorulan koşullar:** Toplu bir istek mevcut `TOPLU_KAPI` sırasına girmeden önce politikaya
  sorulur. Bakılan koşullar duraklatma, saat aralığı, bellek, CPU ve kullanıcı etkinliğidir.
- **İzin yoksa:** İstek reddedilmez. Yaklaşık 5 saniyede bir yeniden bakarak bekler ve izin gelince
  kaldığı yerden devam eder.
- **Görünürlük:** Bekleyen iş, Bağlantılar sekmesindeki canlı kuyrukta yeni bir "Politika bekliyor"
  evresiyle görünür. Takılma çubuğu bu evrede kırmızıya dönmez.
- **Hızlı yol:** Tek ve kısa metinli istekler (≤512 karakter) politikaya hiç sorulmaz. Aramalar ve
  nazard'ın bağlantı yoklaması bu yoldan geçer; bu yüzden duraklatma aramayı hiçbir zaman yavaşlatmaz.
- **Okuma maliyeti:** Politika dosyası her denetimde yalnız değişme tarihine bakılarak yeniden okunur.

### 2.2 Arka plan sekmesi

**Satırlar:** "Toplu gömme" ve "OCR" satırlarının her biri şunları içerir:

- durum ve nedeni ("● çalışıyor", "◐ saat dışı", "⏸ 15:40'a dek"),
- iki üç sayı: gömmede bugün gömülen metin ve Nazar'ın parça kapsamı; OCR'da kuyruk ve bugün işlenen,
- ⏸ düğmesi: basınca "1 saat / bugün boyunca / ben sürdürene dek" seçenekleri açılır;
  duraklatılmışken düğme ▶ Sürdür olur,
- saat seçimi: "her zaman" ya da iki saat kutusu.

**Altta ortak sınırlar:** bellek %, CPU % ve "bilgisayarı kullanırken dur" kutusu.

**Kaydetme:** Her değişiklik anında dosyaya yazılır; ayrı bir "kaydet" düğmesi yoktur.

**egui kuralları (CLAUDE.md):**

- **Dosya işleri:** Okuma ve yazma çizim iş parçacığında yapılmaz. Arka planda yapılır, sonuç
  kanaldan gelir ve kare işlevi `try_recv()` ile okur.
- **Yeniden çizim:** Koşulsuz yeniden çizim yoktur. Pencere durum değişince ya da
  `request_repaint_after(5 s)` ile çizilir.
- **Yerleşim:** Taşmasız yerleşim için `kilim_tema` yardımcıları kullanılır: `alan_satiri`, `birim`,
  açılışta `sarma_kur`. Formda `Grid` kullanılmaz. Bu yardımcılar için kilim-tema rev'i (ve aynı
  rev'i çeken fihrist-tema) güncellenir.
- **Kod yeri:** Sekmenin kodu yeni `bge-arkaplan-sekme` sandığına gider. `bge-gui` zaten 1.154
  satırda; ona yalnız sekme girdisi ve tek bir çağrı eklenir.

### 2.3 Ek düzeltmeler

- **`ac.ps1`:** Masaüstü kısayolunun kullandığı betik yeni süreç ve pencere adını (`ibnun-nedim`)
  arayacak. Böylece kısayol pencereyi yeniden açabilecek; bugün hâlâ `bge-embed-rs` arıyor.
- **Sekme adları:** Dördü birden Türkçe olacak: Durum / Bağlantılar / Ayarlar / Arka plan.

## 3. Nazar tarafı

### 3.1 Yeni sandık `nazar-arkaplan`

`nazar-daemon` sandık sınırında. Bu yüzden arka plan işinin "ne zaman koşacağı" kararı oradan bu
sandığa taşınır. Taşınanlar:

- bugünkü `bosta()` denetimi,
- OCR sırası kararı,
- gömme döngüsündeki bekleme denetimleri,
- yeni durum dosyası yazıcısı ve günlük sayaçlar.

Daemon bu sandığa yalnız iki soru sorar: "OCR koşabilir mi?" ve "gömme koşabilir mi?". Böylece
daemon küçülür ve kararın kendisi tek bir yerde sınanır.

### 3.2 Kurallar

- **Politikadan gelenler:** Elle duraklatma, saat aralığı, bellek, CPU ve kullanıcı etkinliği
  `arkaplan-politika`dan okunur. Bugün kodda sabit duran sınırlar (CPU %50, bellek %90/85)
  politikadaki değerlere bağlanır. Dosya yoksa bugünkü değerlerle aynı varsayılanlar geçerlidir.
- **Nazar'ın kendi koşulları sürer:**
  - son 60 sn'de arama yapılmamış olmalı,
  - indeksleme sürmüyor olmalı,
  - OCR için belge vektörleri bitmiş olmalı.
- **Gömme istekleri tek kapıdan geçer:** Belge, parça ve terim vektörü istekleri gömme izni yoksa
  gönderilmez; 5 saniyede bir yeniden bakılır. Bugün belge vektörleri hiç beklemeden gidiyor.
  - Yeni hâlde "gömmeyi duraklat" gerçekten her şeyi durdurur.
  - Yeni bir belgenin anlam araması da sürdürülene dek bekler.
  - BM25 araması bundan etkilenmez, hemen çalışır.
- **`ocr_saatleri` kaldırılır:** Bu ayar ayardan ve koddan çıkar, yerini politika dosyasının `[ocr]`
  bölümü alır. Kurulumda politika dosyası şu değerlerle oluşturulur ve `ayar.toml`'daki satır silinir:
  - OCR: 00:00–06:30,
  - gömme: "her zaman".

### 3.3 Durum dosyası

- **Yazma:** nazard dosyayı 10 saniyede bir yazar. Yazma önce geçici dosyaya yapılır, sonra adı
  değiştirilir; böylece sekme yarım yazılmış bir dosyayı okumaz.
- **Sayaçlar:** "Bugün işlenen" sayaçları bellekte tutulur ve gece yarısı sıfırlanır. nazard yeniden
  başlarsa o günün sayısı sıfırdan başlar; bunlar kalıcı tutulmaz.

**Kapsam dışı:** Nazar masaüstü uygulaması ve `nazar_durum` MCP aracı değişmez. Arka plan durumu
yalnız ibnü'n-Nedim sekmesinde görünür.

## 4. Hata durumları ve sınama

### 4.1 Hata durumları

| Durum | Davranış |
|---|---|
| Politika dosyası yok | Kısıt yok, bugünkü varsayılanlar geçerli. Sekme "varsayılanlar" yazar; ilk değişiklik dosyayı oluşturur. |
| Dosya bozuk ya da `surum` bilinmiyor | İki süreç de son geçerli politikayla sürer; hiç geçerli politika okunmamışsa varsayılanla sürer. Hata günlüğe yazılır ve sekmede kırmızı bir satır olarak görünür. Dosyanın üzerine yazılmaz; sekmenin ilk değişikliği onu düzeltir. Sessiz yok sayma yoktur. |
| nazard'ın durum dosyası yok ya da 60 sn'den eski | OCR satırı "nazard yanıt vermiyor" der. Düğmeler yine çalışır, çünkü politika dosyası nazard'dan bağımsızdır. |
| Saat ya da CPU ölçülemedi | O koşul engel sayılmaz (bugünkü `ocr_saatinde` gibi). |
| İş politika yüzünden bekliyorken istemci vazgeçti | Mevcut "sahipsiz" işaretiyle görünür; iş, izin gelince hesaplanıp atılır (bugünkü davranış). |

### 4.2 Sınama

- **`arkaplan-politika`:**
  - TOML okuma ve yazma gidiş dönüşü.
  - Bozuk dosya ve bilinmeyen sürüm açık hata verir.
  - Gece yarısını aşan saat aralığı doğru değerlendirilir.
  - Duraklatma bitişi doğru hesaplanır: "1 saat", "bugün boyunca", "ben sürdürene dek".
  - Karar tablosu: her koşul tek başına izni keser ve doğru nedeni döndürür.
  - CPU hesabı arka plan süreçlerini düşer (sahte sayılarla sınanır).
- **Sözleşme sınaması:** Aynı örnek `arkaplan.toml` iki repoda da okunur. Nazar ortak sandığı
  kullandığı için sapma ancak rev farkıyla olabilir; bu sınama onu yakalar.
- **`nazar-arkaplan`:**
  - İzin yoksa OCR sırası boş döner.
  - Gömme kapısı izin gelene dek bekler.
  - Durum dosyası adı değiştirilerek, bütün olarak yazılır.
  - Gece yarısı sayaçlar sıfırlanır.
- **ibnü'n-Nedim kapısı:**
  - Duraklatılmışken toplu istek bekler.
  - Tek kısa istek beklemeden geçer.
  - Sürdürülünce bekleyen istek tamamlanır.
- **Sekme:**
  - Mantık (düğme → dosyaya yazılan değer) arayüzsüz sınanır.
  - Gerçek pencere açılır ve yalnız o pencerenin ekran görüntüsüyle doğrulanır. "Süreç çalışıyor"
    demek pencere içeriğinin doğru olduğunu kanıtlamaz.
- **Kapı ve kurulum:** İki repoda da `kapi .` kırmızısız olur. Kurulumdan sonra şunlar ölçülür:
  - gündüz OCR koşmuyor ("saat dışı"),
  - elle duraklatınca ibnü'n-Nedim'in CPU'su sıfıra iniyor,
  - arama sürüyor.

## Kapsam dışı notlar

- **Render arka ucu:** CLAUDE.md'ye göre egui uygulamaları Windows'ta önce Vulkan'ı dener, yoksa
  DX12'ye düşer. Bu repo ise yalnız DX12 kullanıyor (`Cargo.toml` yorumu). Bu iş ayrı ele alınacak.
- **¶ numaraları:** Nazar'ın docx ¶ numaraları ve terim kökü (önek) araması bu tasarıma girmez.
