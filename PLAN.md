# RUSTBLEND — Geliştirme Planı

> Son plan güncellemesi: 6 Kasım 2026  
> Bu dosya, RUSTBLEND'in teknik yol haritası ve yeni sohbetlerde devamlılık için ana referansıdır.

## 1. Değişmez proje kuralları

1. **Tek dal vardır: `main`.** Yeni dal, feature branch, test branch veya paralel geliştirme dalı açmak kesinlikle yasaktır.
2. Bütün değişiklikler doğrudan `main` üzerine commit edilir. Pull request veya dal tabanlı iş akışı kullanılmaz.
3. Mevcut proje korunur. Gereksiz yere yeni proje oluşturulmaz; çalışan modüller sebepsiz yere yeniden yazılmaz.
4. Bir dosya değiştirilmeden önce mevcut içeriği ve bağımlılıkları incelenir. Büyük değişiklikler küçük, doğrulanabilir adımlara bölünür.
5. Başarılı olduğu gösterilmemiş test, derleme veya cihaz davranışı başarılı kabul edilmez.
6. Gizli anahtarlar, imzalama dosyaları, parolalar, kişisel tokenlar ve yerel SDK/NDK dosyaları public depoya eklenmez.

## 2. Projenin amacı ve hedef ortam

RUSTBLEND, Rust ile geliştirilen, Android telefonlarda çalışan Blender esintili bir 3B modelleme uygulamasıdır. Nihai ürün Termux gerektirmeyen, bağımsız kurulabilir bir Android APK olmalıdır.

İlk doğrulama cihazı:
- Redmi Note 8 Pro
- Android 11
- ARM64
- Mali-G76 MC4

Uygulama portre ve yatay ekranı desteklemeli; ancak mimari yalnızca bu cihaza kilitlenmemelidir. Mobil GPU, bellek, ısı ve dokunmatik kullanım kısıtları tasarımın parçasıdır.

## 3. Mevcut kod tabanı ve bilinen durum

Mevcut mimarinin ana parçaları:

| Dosya / modül | Sorumluluk |
|---|---|
| `src/math.rs`, `src/math/` | Vektör, matris ve quaternion matematiği |
| `src/geometry/` | Mesh, vertex/edge/face ve geometrik özellikler |
| `src/camera.rs` | Kamera matematiği ve hareket altyapısı |
| `src/scene.rs` | Sahne, nesneler, dönüşümler ve seçim durumu |
| `src/render.rs` | Sahneyi GPU'ya aktarılabilecek RenderPacket verisine dönüştürme |
| `src/gpu.rs` | wgpu 30 / GLES renderer altyapısı |
| `src/android_app.rs` | winit Android uygulama döngüsü ve pencere yaşam döngüsü |
| `scripts/build-android-apk.sh` | GitHub runner üzerinde Android ARM64 derleme ve APK paketleme |
| `.github/workflows/android-build.yml` | GitHub Actions test, bulut derlemesi ve APK artifact teslimi |

Bilinen mevcut gerçekler:

- CPU tarafında matematik, geometri, kamera, sahne, seçim ve RenderPacket altyapısı oluşturulmuştur.
- GPU öncesi sürümde 81 testin geçtiği kaydedilmiştir. Bu sonuç, GPU eklendikten sonraki güncel kaynak için otomatik olarak geçerli sayılmaz.
- wgpu 30.0.1 ve winit 0.30.13 ile Android ARM64 release derlemesi başarılı olmuştur.
- Önceki 0.2.0 renderer'ı yalnızca `gpu.rs` içinde koordinatları elle yazılmış WGSL test üçgenini çiziyordu; kullanıcı bu üçgenin Redmi Note 8 Pro ekranında göründüğünü ekran görüntüsüyle doğruladı.
- 0.3.0 renderer'ı `RenderPacket::from_scene` çıktısını GPU vertex/index buffer'larına aktarır; başlangıç sahnesinde gerçek `Mesh::cube(2.0)` geometrisi, model dönüşümü, kamera view-projection matrisi ve depth buffer kullanılır.
- 0.3.0 viewport ayrıca XZ zemin grid'i, X/Y/Z dünya eksenleri ve seçili nesne için turuncu kenar çizgileri üretir. Bir parmakla orbit, iki parmakla pinch zoom dokunmatik girdisi eklendi.
- Kullanıcı, 0.2.0 APK'sını Redmi Note 8 Pro / Android 11 üzerinde açtığında yalnızca siyah ekran gördüğünü bildirdi. Bu nedenle önceki cihaz doğrulaması **başarısızdır**; test üçgeninin göründüğü kabul edilmez.
- İncelemede, GPU başlatma hatasının yalnızca Android loguna yazıldığı ve uygulamanın renderer olmadan boş pencerede kaldığı görüldü. İlk kare alma denemesi de yalnızca bir kez isteniyordu.
- 6 Kasım 2026 düzeltmesi: `gpu.rs` Vulkan + GLES backend'lerini etkinleştirir, FIFO sunum modunu kullanır ve GPU başlangıç adımlarını loglar. `android_app.rs` renderer çalışırken VSync kontrollü sürekli redraw döngüsü kullanır. Bu değişikliklerin derlenmesi cihazda görüntünün kesin düzeldiğini kanıtlamaz.
- `render.rs` içindeki RenderPacket henüz GPU vertex/index buffer'larına bağlanmamıştır.
- Kamera ve sahne sistemi henüz Android viewport çizim döngüsüne bağlanmamıştır.
- Android uygulama kabuğunun ve GPU yüzeyinin gerçek cihazdaki çalışması ayrıca doğrulanmalıdır.
- Mevcut Android GPU instance'ı Vulkan ve GLES backend'lerini birlikte etkinleştirir; gerçek cihazda seçilen backend adapter logunda görünür. 0.3.0 ekran testi henüz beklenmektedir.
- `AndroidManifest.xml` sürüm bilgisi 0.2.0 / versionCode 2 iken `build_apk.sh` içinde AAPT2 için 0.1.0 / versionCode 1 sabitlenmiştir. Bu tutarsızlık düzeltilmelidir.
- Eski `build_apk.sh` Termux'a özel ve kullanımdan kaldırılmıştır; aktif geliştirme veya APK teslim akışında kullanılmaz.
- Aktif `scripts/build-android-apk.sh` betiği GitHub runner'ın SDK/NDK ortamını kullanır, sürüm bilgisini `AndroidManifest.xml` dosyasından okur ve imza/hizalama/paket metadata kontrolleri yapar.
- CI imza anahtarı runner üzerinde geçici olarak üretilir; depoya veya APK artifact'ine eklenmez. Bu nedenle ilk CI APK'sı eski yerel debug APK'nın üzerine güncellenemez.
- Yerel `.bak` dosyaları ilk GitHub aktarımına dahil edilmemiştir. İçerikleri incelenmeden aktif kaynak koduna veya depoya geri eklenmez.

## 4. Öncelik: Aşama 0 — sağlamlaştırma

Yeni modelleme özelliklerine başlamadan önce mevcut temel doğrulanacaktır.

### 0.1. Kaynak ve depo bütünlüğü

- [ ] GitHub deposunun public olduğunu ve varsayılan dalın `main` olduğunu kontrol et.
- [ ] Uzak depo adresini doğrula: `https://github.com/mehmetdem2005/Rustblend.git`.
- [ ] Yalnızca `main` dalının bulunduğunu kontrol et; yeni dal oluşturma.
- [ ] Yerel çalışma ağacının temiz olduğunu ve beklenmeyen dosya değişiklikleri bulunmadığını doğrula.
- [x] Geliştirme ve APK derleme akışını Termux bağımlılığından çıkarıp GitHub Actions'a taşıyacak CI dosyalarını ekle.
- [ ] `.gitignore` kurallarının keystore, yerel ortam dosyası, SDK/NDK ve build çıktısını dışarıda tuttuğunu kontrol et.
- [ ] Yerel `.bak` dosyalarını yalnızca karşılaştırma amacıyla incele; gerekli bir kod veya karar varsa kontrollü biçimde değerlendir.

### 0.2. Test ve derleme taban çizgisi

- [x] GitHub Actions run #10 üzerinde `cargo test --locked`: **81 test geçti, 0 başarısız** (6 Kasım 2026).
- [ ] Hataları sınıflandır ve düzelt; testleri atlama veya başarısız testi başarılı gibi işaretleme.
- [x] GitHub Actions run #10 üzerinde Android ARM64 release cross-compile başarılı.
- [x] GitHub Actions run #16 üzerinde güncel Vulkan + GLES renderer ve Android redraw döngüsü Android ARM64 için derlendi; 81/81 test geçti, APK üretildi.
- [ ] Kullanılan Rust, wgpu, winit, SDK ve NDK sürümlerini gerektiğinde kaydet.
- [ ] Gereksiz bağımlılık ekleme; eklenen her bağımlılığın gerekçesini belgele.

### 0.3. APK üretim hattını düzeltme

- [x] Yeni GitHub Actions APK betiği sürüm numarasını doğrudan `AndroidManifest.xml` dosyasından okur.
- [x] Eski Termux-only `build_apk.sh` betiği emekliye ayrıldı; aktif APK akışında kullanılmıyor.
- [x] Sürüm bilgisi aktif cloud build betiğinde doğrudan `AndroidManifest.xml` dosyasından okunur.
- [x] APK paketinde `com.rustblend.mobile`, ARM64 `libmain.so`, `ANativeActivity_onCreate` ve `android_main` doğrulandı.
- [x] `apksigner verify` başarılı; v1, v2 ve v3 imza şemaları doğrulandı.
- [x] ZIP hizalaması ve ELF LOAD segmentlerinin tamamında 16 KB hizalama doğrulandı.
- [x] Paket metadata'sı doğrulandı: versionCode 2, versionName 0.2.0, minSdk 23, targetSdk 30.
- [x] CI geçici imza anahtarını runner üzerinde üretir; özel anahtar depoya eklenmez.
- [ ] Mevcut CI her çalışmada yeni geçici anahtar ürettiği için her yeni APK temiz kurulum gerektirir.
- [ ] Güncelleme kurulumlarını desteklemek için kalıcı imza keystore'unu GitHub Actions Secrets üzerinden yapılandır; hiçbir zaman public depoya ekleme.
- [ ] Keystore veya parolasını hiçbir koşulda Git'e ekleme.

### 0.4. GitHub Actions ve APK teslimi

- [x] `.github/workflows/android-build.yml` dosyası eklendi; `main` üzerine push ve manuel workflow_dispatch ile çalışır.
- [x] `scripts/build-android-apk.sh` Linux GitHub runner üzerinde Android ARM64 derleme, NativeActivity sembolü, ELF 16 KB segment hizalaması, APK metadata, imza ve ZIP hizalama kontrolleri için eklendi.
- [x] CI sonucu `CI_BUILD_STATUS.json` içine kaydedilecek; başarılı sonuç artifact kimliğini, başarısız sonuç GitHub Actions run bağlantısını içerir.
- [x] GitHub Actions run #10 tüm adımları geçti: test, Android ARM64 cross-compile, APK imza ve hizalama.
- [x] APK artifact indirildi ve arşivden çıkarılarak yerel dosya bütünlüğü kontrol edildi; artifact ID `13532651312`, saklama süresi 14 gün.
- [x] Teslim edilen dosya: `RUSTBLEND-0.2.0-arm64.apk` (8,5 MB). SHA-256: `2fcea8d738f5141e7f320ffa3340d86591c069da4ef1c45e3fed522e55ba4586`.
- [x] Düzeltilmiş kaynak kodundan üretilen yeni aday APK artifact'i: run #16, artifact ID `13535580631`; boyut 8,85 MB. CI imza, ZIP hizalama ve paket metadata kontrolleri başarılı.

### 0.5. Gerçek cihaz doğrulaması

**Cihaz testi kaydı — 6 Kasım 2026:** Önceki 0.2.0 APK'sı siyah ekran verdi (kullanıcı bildirimi). Aşağıdaki kontroller yeni aday APK için yeniden yapılmalıdır; eski build'in CI başarısı bu kontrolün yerine geçmez.

- [ ] APK'yı Redmi Note 8 Pro / Android 11 üzerine kur veya aynı imza anahtarıyla güncelle.
- [ ] Uygulamanın çökmeden açıldığını doğrula.
- [ ] GPU adapter bilgisini ve Android loglarını kaydet.
- [ ] Test üçgeninin gerçekten ekranda göründüğünü doğrula; yalnızca başarılı derleme yeterli değildir.
- [ ] Uygulama arka plana alınıp geri getirildiğinde yüzeyin ve renderer'ın yeniden çalıştığını dene.
- [ ] Portre/yatay yön değişimini ve pencere yeniden boyutlandırmasını dene.
- [ ] Yüzey kaybı, Outdated/Lost durumları ve yeniden yapılandırma davranışını gözlemle.
- [ ] Sonuçları bu dosyadaki kontrol listesine ve commit açıklamasına işle.

**Aşama 0 tamamlanma ölçütü:** Güncel testler geçmeli, release derlemesi başarılı olmalı, APK imza/hizalama/metadata kontrollerinden geçmeli ve gerçek telefonda uygulama ile test görüntüsü doğrulanmalıdır. Bu koşullar sağlanmadan 3B modelleme özelliklerine geçilmez.

## 5. Aşama 1 — gerçek 3B viewport

0.3.0 için GitHub Actions run #20 üzerinde aşağıdaki ilk viewport parçaları Android ARM64'e derlenmiştir:

- [x] `RenderPacket` verisini GPU vertex ve index buffer'larına aktar.
- [x] Nesnelerin model matrislerini GPU'ya aktarılmadan önce mesh konumlarına uygula.
- [x] Mevcut kamera view/projection matrislerini viewport çizimine bağla.
- [x] Derinlik tamponu ve yüzey yeniden boyutlandırmasını ekle.
- [x] Başlangıç sahnesinde gerçek `Mesh::cube(2.0)` geometrisini çiz.
- [x] Zemin grid'i ve X/Y/Z dünya eksenlerini ekle.
- [x] Seçili nesnenin kenar çizgilerini vurgula.
- [x] Tek parmak orbit ve iki parmak pinch zoom kontrolünü ekle.
- [ ] Sahne değiştiğinde GPU vertex/index buffer'larını kontrollü biçimde güncelle.
- [ ] Bellek ve kaynak ömrünü yönet; her frame gereksiz buffer oluşturma.
- [ ] Redmi Note 8 Pro'da 0.3.0 viewport görüntüsünü, dokunmatiği ve performansı doğrula.

**Kabul ölçütü:** CPU sahnesindeki nesneler doğru dönüşümle GPU'da görünmeli; kamera hareketi görüntüyü güncellemeli; uygulama yaşam döngüsü testleri çalışmaya devam etmelidir. Kaynak kodunun derlenmesi cihazdaki görüntünün doğrulandığı anlamına gelmez.

## 6. Aşama 2 — dokunmatik etkileşim ve seçim

- [ ] Tek parmakla orbit/pan davranışlarını belirle ve uygula.
- [ ] İki parmakla zoom/pinch desteği ekle.
- [ ] Ekran koordinatını dünya ışınına dönüştürerek nesne seçiminin temelini oluştur.
- [ ] Tekli, çoklu ve aktif nesne seçimini mevcut Scene API'siyle eşleştir.
- [ ] Dokunmatik alanların viewport ile çakışmamasını sağla.
- [ ] Farklı ekran oranlarında kontrol hissini test et.

**Kabul ölçütü:** Kullanıcı sahneyi rahatça çevirebilmeli, yakınlaştırabilmeli, kaydırabilmeli ve görünür nesneleri seçebilmelidir.

## 7. Aşama 3 — temel modelleme araçları

- [ ] Nesne ekleme ve silme.
- [ ] Taşıma, döndürme ve ölçekleme araçları.
- [ ] Transform gizmo ve sayısal değer girişi.
- [ ] Vertex / edge / face seçim modları.
- [ ] Temel mesh düzenleme işlemleri.
- [ ] Undo/redo için işlem geçmişi tasarımı.
- [ ] Bozuk veya geçersiz geometriye karşı kontroller ve testler.

**Kabul ölçütü:** Kullanıcı temel bir nesne oluşturup seçebilmeli, dönüştürebilmeli ve mesh geometrisini kontrollü biçimde değiştirebilmelidir.

## 8. Aşama 4 — proje dosyaları ve ileri özellikler

- [ ] GLB/GLTF içe aktarma.
- [ ] GLB/GLTF dışa aktarma.
- [ ] Yerel proje/sahne kaydetme ve yeniden açma.
- [ ] Basit materyal ve ışıklandırma sistemi.
- [ ] Nesne listesi / outliner.
- [ ] Daha gelişmiş modelleme araçları.
- [ ] Büyük sahneler için performans ve bellek iyileştirmeleri.

## 9. Her değişiklik için kalite kapısı

Her önemli değişiklikte aşağıdaki sıra izlenir:

1. Mevcut dosyaları ve mimariyi incele.
2. Değişikliğin kapsamını ve etkilediği modülleri belirle.
3. Küçük ve kontrollü değişiklik yap.
4. Uygun testleri çalıştır; mümkünse `cargo test` ile tam test paketi çalıştır.
5. `cargo build --release` sonucunu doğrula.
6. APK gerekiyorsa üret, imza/hizalama/metadata kontrollerini yap.
7. Cihaz üzerinde etkilenen davranışı doğrula.
8. Değişikliği doğrudan `main` dalına commit et.
9. Bu plandaki durum ve doğrulama notlarını güncelle.

Derleme başarısı, cihazda çalışma başarısı anlamına gelmez. Ekranda görüntü alınması da modelleme özelliğinin tamamlandığı anlamına gelmez. Her kabul ölçütü ayrı doğrulanır.

## 10. Yeni sohbetler ve katkıda bulunanlar için talimat

RUSTBLEND üzerinde çalışmaya başlamadan önce `README.md` ve bu `PLAN.md` dosyasını oku. Mevcut kodu incelemeden dosya değiştirme. Tek dal kuralını ihlal etme: bütün çalışmalar doğrudan `main` üzerinde yapılır. İlk öncelik Aşama 0 sağlamlaştırma kontrol listesidir; bu tamamlanmadan yeni özellik geliştirmeye başlanmaz.

---

## Güncel ilerleme kaydı

- [x] Public GitHub deposu oluşturuldu.
- [x] İlk kaynak aktarımı `main` dalına yapıldı.
- [x] Tek dal kuralı `README.md` içine yazıldı.
- [x] Kaynak kodu, `Cargo.toml`, `Cargo.lock`, manifest ve APK betiği depoya aktarıldı.
- [x] GitHub Actions tabanlı bulut test/derleme iş akışını ekle.
- [x] Aşama 0: GitHub Actions üzerinde 81/81 Rust testi geçti.
- [x] Aşama 0: GitHub runner Android ARM64 release library üretti.
- [x] Aşama 0: 0.2.0 APK üretildi; imza, metadata ve hizalama doğrulamaları geçti.
- [x] Aşama 0: 0.2.0 düzeltme APK'sında test üçgeni kullanıcının ekran görüntüsüyle doğrulandı; ilk 0.2.0 APK siyah ekran verdiği için başarısız deneme olarak kaydedildi.
- [ ] Aşama 1: 0.3.0 gerçek sahne viewport'unu Redmi Note 8 Pro'ya kurup küp, grid, eksenler ve dokunmatik orbit/zoom davranışını doğrula.

Termux artık geliştirme sürecinin parçası değildir. Kod değişiklikleri doğrudan `main` üzerinde GitHub üzerinden yapılır; test ve APK derlemesi GitHub Actions'ta çalışır; başarılı APK artifact olarak teslim edilir.

Bu liste yalnızca gerçek çıktılar görüldüğünde güncellenir.
