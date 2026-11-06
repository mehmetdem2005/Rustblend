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
| `build_apk.sh` | Rust kütüphanesini derleme ve APK paketleme |

Bilinen mevcut gerçekler:

- CPU tarafında matematik, geometri, kamera, sahne, seçim ve RenderPacket altyapısı oluşturulmuştur.
- GPU öncesi sürümde 81 testin geçtiği kaydedilmiştir. Bu sonuç, GPU eklendikten sonraki güncel kaynak için otomatik olarak geçerli sayılmaz.
- wgpu 30.0.1 ve winit 0.30.13 ile Android ARM64 release derlemesi başarılı olmuştur.
- Mevcut `gpu.rs` gerçek sahne geometrisini henüz çizmiyor; WGSL shader ile test üçgeni çiziyor.
- `render.rs` içindeki RenderPacket henüz GPU vertex/index buffer'larına bağlanmamıştır.
- Kamera ve sahne sistemi henüz Android viewport çizim döngüsüne bağlanmamıştır.
- Android uygulama kabuğunun ve GPU yüzeyinin gerçek cihazdaki çalışması ayrıca doğrulanmalıdır.
- Vulkan kabiliyeti gerçek Mali sürücüsünde doğrulanmadığından mevcut renderer GLES backend'ini seçmektedir.
- `AndroidManifest.xml` sürüm bilgisi 0.2.0 / versionCode 2 iken `build_apk.sh` içinde AAPT2 için 0.1.0 / versionCode 1 sabitlenmiştir. Bu tutarsızlık düzeltilmelidir.
- APK betiği yerel `~/.rustblend-env` dosyasına ve geliştirici makinesindeki Android SDK/NDK araçlarına ihtiyaç duyar. Bu yerel ortam dosyası depoya eklenmez.
- Debug imzalama anahtarı `~/.android/rustblend-debug.keystore` altında tutulur; public depoya konmaz.
- Yerel `.bak` dosyaları ilk GitHub aktarımına dahil edilmemiştir. İçerikleri incelenmeden aktif kaynak koduna veya depoya geri eklenmez.

## 4. Öncelik: Aşama 0 — sağlamlaştırma

Yeni modelleme özelliklerine başlamadan önce mevcut temel doğrulanacaktır.

### 0.1. Kaynak ve depo bütünlüğü

- [ ] GitHub deposunun public olduğunu ve varsayılan dalın `main` olduğunu kontrol et.
- [ ] Uzak depo adresini doğrula: `https://github.com/mehmetdem2005/Rustblend.git`.
- [ ] Yalnızca `main` dalının bulunduğunu kontrol et; yeni dal oluşturma.
- [ ] Yerel çalışma ağacının temiz olduğunu ve beklenmeyen dosya değişiklikleri bulunmadığını doğrula.
- [ ] `.gitignore` kurallarının keystore, yerel ortam dosyası, SDK/NDK ve build çıktısını dışarıda tuttuğunu kontrol et.
- [ ] Yerel `.bak` dosyalarını yalnızca karşılaştırma amacıyla incele; gerekli bir kod veya karar varsa kontrollü biçimde değerlendir.

### 0.2. Test ve derleme taban çizgisi

- [ ] Güncel kaynak üzerinde `cargo test` çalıştır; GPU eklendikten sonraki ilk gerçek test sonucunu kaydet.
- [ ] Hataları sınıflandır ve düzelt; testleri atlama veya başarısız testi başarılı gibi işaretleme.
- [ ] `cargo build --release` ile Android hedef derlemesini doğrula.
- [ ] Kullanılan Rust, wgpu, winit, SDK ve NDK sürümlerini gerektiğinde kaydet.
- [ ] Gereksiz bağımlılık ekleme; eklenen her bağımlılığın gerekçesini belgele.

### 0.3. APK üretim hattını düzeltme

- [ ] `AndroidManifest.xml` ile `build_apk.sh` içindeki versionCode ve versionName değerlerini tutarlı hale getir.
- [ ] Sürüm bilgisinin iki ayrı yerde elle ve farklı değerlerle kalmasını önleyecek bir yöntem belirle.
- [ ] APK paketinde `com.rustblend.mobile`, ARM64 `libmain.so` ve Android giriş sembollerinin bulunduğunu doğrula.
- [ ] APK imzasını `apksigner verify` ile, 16 KB hizalamasını `zipalign -c -P 16` ile kontrol et.
- [ ] APK içindeki gerçek versionCode/versionName ve minSdk/targetSdk değerlerini paket metadata'sından doğrula.
- [ ] Mevcut debug keystore'u koru. Aynı anahtarla imzalanan APK'yı eski sürümün üzerine kur; test için uygulamayı gereksiz yere kaldırma.
- [ ] Keystore veya parolasını hiçbir koşulda Git'e ekleme.

### 0.4. Gerçek cihaz doğrulaması

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

Aşama 0 tamamlandıktan sonra:

- [ ] `RenderPacket` verisini GPU vertex ve index buffer'larına aktar.
- [ ] Nesnelerin model matrislerini shader'a gönder.
- [ ] Mevcut kamera view/projection matrislerini viewport çizimine bağla.
- [ ] Derinlik tamponu ve doğru yüzey formatı/viewport ayarlarını ekle.
- [ ] Önce tek bir test küpünü, sonra birden fazla mesh'i çiz.
- [ ] Grid ve dünya eksenlerini ekle.
- [ ] Kaynak mesh değiştiğinde GPU buffer'larının güncellenmesini sağla.
- [ ] Bellek ve kaynak ömrünü yönet; her frame gereksiz buffer oluşturma.
- [ ] Temel çizim doğruluğunu ve cihaz performansını ölç.

**Kabul ölçütü:** CPU sahnesindeki nesneler doğru dönüşümle GPU'da görünmeli; kamera değişince görüntü güncellenmeli; uygulama yaşam döngüsü testleri çalışmaya devam etmelidir.

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
- [ ] Aşama 0: güncel kaynakta testleri tekrar çalıştır.
- [ ] Aşama 0: APK sürüm metadata tutarsızlığını gider.
- [ ] Aşama 0: güncel APK'yı üret ve doğrula.
- [ ] Aşama 0: gerçek cihazda GPU görüntüsünü ve yaşam döngüsünü doğrula.

Bu liste yalnızca gerçek çıktılar görüldüğünde güncellenir.
