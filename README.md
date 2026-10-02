# ReceiptBox

> **Your documents. Your data. Your device.**

ReceiptBox; makbuz, invoice, warranty ve diğer finansal dokümanları kendi cihazınızda saklamanız için geliştirilen local-first, open-source bir desktop application’dır.

Proje [muhammedkoca.com.tr](https://muhammedkoca.com.tr) tarafından geliştirilmiştir. ReceiptBox açık kaynaklıdır; kaynak kodu incelenebilir, fork edilebilir ve MIT License koşullarıyla geliştirilebilir.

> ReceiptBox sertifikalı bir muhasebe ürünü değildir ve tax advice sağlamaz.

## Neden ReceiptBox?

- **Local-first:** Dokümanlarınız varsayılan olarak cihazınızda kalır.
- **No account required:** Zorunlu hesap, subscription veya cloud storage yoktur.
- **Privacy-oriented:** Doküman işleme için zorunlu upload ya da telemetry yoktur.
- **Searchable archive:** SQLite FTS5 ile metadata ve OCR text araması için altyapı vardır.
- **Open source:** Nasıl çalıştığını görebilir, geliştirebilir ve kendi build’inizi üretebilirsiniz.

## Mevcut özellikler

- Tauri + React + TypeScript desktop foundation
- Native file picker ile PDF, JPG/JPEG, PNG, WebP ve TIFF import
- File signature ve 25 MiB size-limit validation
- SHA-256 content hashing ve exact duplicate detection
- Application-generated local original-file storage
- SQLite metadata storage ve FTS5 search index
- Document review: merchant, date, total, currency, category ve OCR text düzenleme
- CSV export; spreadsheet formula-injection protection içerir
- Versioned backup manifest foundation
- Tesseract runtime availability check
- Responsive, keyboard-friendly document management UI

## Şu an henüz tamamlanmayanlar

ReceiptBox aktif geliştirme aşamasındadır. Aşağıdakiler henüz release-ready değildir:

- Packaged Tesseract OCR ve multi-page OCR processing
- PDF text extraction/rendering ve gerçek preview
- Backup restore, integrity verification ve overwrite confirmation
- Advanced reports, warranties, app trash ve encryption-at-rest
- Signed Windows/macOS/Linux installers

Bu özellikler tamamlanana kadar uygulamayı kritik muhasebe arşivinizin tek kopyası olarak kullanmayın; bağımsız backup tutun.

## API key yaklaşımı

ReceiptBox’ın core features’ları API key gerektirmez. OCR ve doküman yönetimi local olarak tasarlanır.

İleride optional online AI integration eklenirse:

- API key yalnızca kullanıcı açıkça eklediğinde kullanılır.
- Key source code, Git history, log, export veya SQLite metadata içine yazılmaz.
- Kullanıcı kendi key’ini kullanır ve feature API key olmadan kapalı kalır.
- Secret değerleri `.env` dosyasında tutulur; `.env` Git’e commit edilmez.

## Quick start

### Requirements

- Node.js 22+
- Rust stable toolchain
- Windows için WebView2 Runtime
- OCR geliştirmesi için isteğe bağlı Tesseract (`eng` ve `tur` language data)

### Development

```powershell
npm install
npm run tauri dev
```

### Verification

```powershell
npm run lint
npm run test
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
npm run tauri build -- --no-bundle
```

Release build çıktısı Windows’ta `src-tauri/target/release/receiptbox.exe` altında oluşturulur. `--no-bundle`, executable build’i doğrular; installer üretmez.

## Local data location

Native app, dokümanları ve SQLite database’i işletim sisteminin application-data alanı altındaki `ReceiptBoxLibrary` klasöründe tutar. Original files database blob’u olarak saklanmaz; application-generated ID ile `originals/` dizinine kopyalanır.

## Project structure

```text
src/                 React UI
src-tauri/src/       Rust commands, SQLite ve storage logic
src-tauri/icons/     Application icons
.github/workflows/   CI workflow
ARCHITECTURE.md      Teknik mimari
SECURITY.md          Security boundary ve bilinen riskler
PRIVACY.md           Privacy policy / local-data yaklaşımı
OCR.md               OCR runtime stratejisi
```

## Security & privacy

Lütfen [SECURITY.md](SECURITY.md), [PRIVACY.md](PRIVACY.md), [OCR.md](OCR.md) ve [ARCHITECTURE.md](ARCHITECTURE.md) dosyalarını okuyun. Bug report içine gerçek makbuz, invoice, API key veya kişisel doküman eklemeyin.

## Contributing

Issues ve pull request’ler memnuniyetle karşılanır.

1. Repository’yi fork edin.
2. Küçük ve odaklı bir branch oluşturun.
3. `npm run lint`, `npm run test` ve ilgili Rust testlerini çalıştırın.
4. Açık bir açıklama ile pull request gönderin.

Security issue’ları public issue olarak paylaşmayın; önce [SECURITY.md](SECURITY.md) içindeki kuralları takip edin.

## License

MIT License © 2026 ReceiptBox contributors.

Made with care by [muhammedkoca.com.tr](https://muhammedkoca.com.tr).
