<div align="center">

<img src="docs/brand/branchcut-icon.svg" alt="Branchcut icon" width="112">

# Branchcut

**क्वेरी कंपाइल करें। ट्री छाँटें।**

<p>
  <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/Rust-2024-DEA584?style=for-the-badge&amp;logo=rust&amp;logoColor=white&amp;labelColor=0A1220" alt="Rust 2024" /></a>
  <img src="https://img.shields.io/badge/core_crates-zero-00C853?style=for-the-badge&amp;labelColor=0A1220" alt="Zero third-party crates in the core" />
  <a href="https://github.com/codex-mohan/branchcut/actions/workflows/install-matrix.yml"><img src="https://img.shields.io/github/actions/workflow/status/codex-mohan/branchcut/install-matrix.yml?style=for-the-badge&amp;label=installers&amp;labelColor=0A1220" alt="Installer checks" /></a>
  <a href="https://github.com/codex-mohan/branchcut/blob/master/LICENSE"><img src="https://img.shields.io/badge/license-MIT-BB86FC?style=for-the-badge&amp;labelColor=0A1220" alt="MIT license" /></a>
  <a href="https://github.com/codex-mohan/branchcut/stargazers"><img src="https://img.shields.io/github/stars/codex-mohan/branchcut?style=for-the-badge&amp;labelColor=0A1220&amp;color=FFD700" alt="GitHub stars" /></a>
  <a href="https://github.com/codex-mohan/branchcut/pulls"><img src="https://img.shields.io/badge/PRs-welcome-00C853?style=for-the-badge&amp;labelColor=0A1220" alt="PRs welcome" /></a>
</p>

<p align="center">
  <a href="README.md">English</a> ·
  <a href="README.es.md">Español</a> ·
  <a href="README.fr.md">Français</a> ·
  <a href="README.zh-CN.md">简体中文</a> ·
  <a href="README.ja.md">日本語</a> ·
  <strong>हिन्दी</strong>
</p>

</div>

Branchcut रोज़मर्रा की खोज, डेवलपमेंट और स्क्रिप्ट के लिए एक नेटिव फ़ाइल सिस्टम क्वेरी इंजन है। यह पैटर्न, बाहर रखने की शर्तों, अनदेखा करने के नियमों, एंट्री फ़िल्टर और रुकने की शर्तों को एक ही डायरेक्टरी स्कैन योजना में कंपाइल करता है। यह उन डायरेक्टरियों को छोड़ सकता है जिनमें कोई मिलान संभव नहीं है, मिलते हुए पाथ तुरंत भेज सकता है और बता सकता है कि स्कैन के दौरान क्या काम हुआ।

लाइब्रेरी और CLI को अलग-अलग ज़िम्मेदारियों वाले Rust मॉड्यूल में बाँटा गया है। कोर इंजन और सामान्य CLI केवल Rust की स्टैंडर्ड लाइब्रेरी का उपयोग करते हैं। रनटाइम डिपेंडेंसी ग्राफ़ में कोई थर्ड-पार्टी क्रेट नहीं है।

[वेबसाइट और दस्तावेज़](https://codex-mohan.github.io/branchcut/) · [रिलीज़](https://github.com/codex-mohan/branchcut/releases) · [सुविधाओं की सूची](docs/reference/feature-status.md) · [बग रिपोर्ट करें](https://github.com/codex-mohan/branchcut/issues)

## इंस्टॉलेशन

[GitHub Releases](https://github.com/codex-mohan/branchcut/releases/latest) से अपने सिस्टम के लिए आर्काइव डाउनलोड करें, `SHA256SUMS` से उसकी पुष्टि करें, उसे एक्सट्रैक्ट करें और एक्ज़ीक्यूटेबल को अपने `PATH` में उपलब्ध करें। पहले से बनी बाइनरी चलाने के लिए Rust या Cargo की ज़रूरत नहीं है। [आर्काइव इंस्टॉल करने के निर्देश](docs/contributing/releases.md#verify-and-install-an-archive) देखें।

Cargo उपलब्ध न हो, तो इंस्टॉलेशन स्क्रिप्ट Rust सेट अप कर सकती हैं। सीधे Cargo से इंस्टॉल करने के लिए Rust टूलचेन पहले से मौजूद होना चाहिए। नीचे दिए गए हर तरीके में Branchcut सोर्स कोड से बिल्ड होता है।

**Windows PowerShell**

```powershell
irm https://raw.githubusercontent.com/codex-mohan/branchcut/master/scripts/install.ps1 | iex
```

**Linux और macOS**

```sh
curl --proto '=https' --tlsv1.2 -fsSL https://raw.githubusercontent.com/codex-mohan/branchcut/master/scripts/install.sh | sh
```

**Cargo**

```sh
cargo install --locked --git https://github.com/codex-mohan/branchcut.git
```

जाँचें कि कमांड उपलब्ध है।

```sh
branchcut --version
branchcut --help
```

स्क्रिप्ट रिलीज़ बिल्ड इंस्टॉल करती हैं। अपनी पसंद की जगह इंस्टॉल करने, अनइंस्टॉल करने और PATH की समस्याएँ हल करने के लिए [इंस्टॉलेशन गाइड](docs/getting-started/installation.md) देखें।

## इस्तेमाल

मौजूदा डायरेक्टरी के भीतर Rust सोर्स फ़ाइलें खोजें।

```sh
branchcut --glob 'src/**/*.rs'
```

सादा टेक्स्ट देने पर वह फ़ाइल नामों में अक्षरशः खोजा जाता है।

```sh
branchcut config
```

एक ही क्वेरी में पैटर्न, बाहर रखने की शर्तें, फ़ाइल फ़िल्टर और नतीजों की सीमा लगाएँ।

```sh
branchcut \
  --glob 'packages/**/src/**/*.{rs,ts}' \
  --exclude '**/{target,node_modules,dist}/**' \
  --type file \
  --limit 100
```

स्कैन की योजना और स्कैन के दौरान असल में दर्ज हुई गिनती देखें।

```sh
branchcut --glob '**/*.rs' --explain
branchcut --glob '**/*.rs' --stats
```

### रोज़मर्रा की खोज और सहेजी गई क्वेरी

सामान्य सोर्स और कॉन्फ़िग फ़ाइलें खोजें। खोज में अनदेखा करने के नियम लागू होंगे और छिपे हुए पाथ भी शामिल होंगे।

```sh
branchcut --glob '**/*.{rs,toml}' --ignore --hidden
```

आकार और आखिरी बदलाव के समय के आधार पर खोजें।

```sh
branchcut --glob '**/*' --size '>=1.5MiB' --modified-within 30d
branchcut --glob '**/*.log' --modified-before '2025-01-01T12:00:00+05:30'
```

प्रोजेक्ट में बार-बार की जाने वाली खोजें `.branchcut-queries` में सहेजें।

```text
[workspace]
glob packages/**/src/**/*.{rs,ts}
exclude **/target/**
ignore true
max-depth 8
```

फिर प्रोफ़ाइल चलाएँ।

```sh
branchcut --config .branchcut-queries --profile workspace --explain
```

और उदाहरणों के लिए [रोज़मर्रा की खोज](docs/guides/everyday-search.md), [अनदेखा करने के नियम और छिपे हुए पाथ](docs/guides/ignore-and-hidden.md) और [सहेजी गई प्रोजेक्ट क्वेरी](docs/guides/project-queries.md) देखें।

### पाइपलाइन और कमांड

किसी दूसरे प्रोग्राम को Unix के किसी भी वैध फ़ाइल नाम को भेजने के लिए पाथ के बीच NUL विभाजक रखें।

```sh
branchcut --glob '**/*.rs' --print0 | xargs -0 rustfmt
```

पाइपलाइन में आगे के टूल को संरचित रूप में पाथ चाहिए, तो JSON Lines का उपयोग करें। फ़ाइल सिस्टम का विवरण जोड़ने के लिए `--json-metadata` लगाएँ।

```sh
branchcut --glob '**/*.log' --json-metadata
```

हर नतीजे को आर्ग्युमेंट के तौर पर देकर कमांड चलाएँ। शेल पाथ की व्याख्या नहीं करेगा।

```sh
branchcut --glob '**/*.tmp' --exec 'rm {}'
```

नतीजे बहुत हों, तो `--exec-batch` पाथ को ऐसे समूहों में बाँटता है जिनका आकार सीमित है। कमांड चलाए बिना आर्ग्युमेंट सूची (argv) देखने के लिए `--exec-preview` लगाएँ। प्लेसहोल्डर, प्लेटफ़ॉर्म के विवरण और विफलता की स्थिति में व्यवहार के लिए [आउटपुट और कमांड चलाना](docs/guides/output-and-exec.md) देखें।

## सुविधाएँ

- **ग्लॉब खोज** में `*`, `?`, `**`, वर्ण वर्ग, नेस्टेड ब्रेस, सीमित रेंज और पाथ के एक हिस्से तक सीमित extglob का समर्थन है।
- **संयुक्त क्वेरी** में शामिल करने वाले कई पैटर्न, बाहर रखने की शर्तें, एक्सटेंशन, प्रकार, गहराई, अक्षरशः खोजे जाने वाले शब्द और मेटाडेटा फ़िल्टर साथ उपयोग कर सकते हैं।
- **सामग्री खोज** में `--content` (उपनाम `--grep`) से चुनी गई फ़ाइलों के अंदर अक्षरशः टेक्स्ट खोजा जा सकता है। इसमें कई पैटर्न OR से जुड़ते हैं, आउटपुट `पाथ:पंक्ति:स्तंभ:टेक्स्ट` होता है, `-l`, `--count`, केस मोड, `--text` से बाइनरी फ़ाइलों को भी पढ़ना और जल्दी रुकना शामिल है। रेगुलर एक्सप्रेशन या बहु-पंक्ति मोड नहीं हैं।
- **अनदेखा करने के विकल्प** में छिपे हुए पाथ डिफ़ॉल्ट रूप से बाहर रहते हैं। Git के इग्नोर स्रोत, `.ignore` या अपनी बताई हुई इग्नोर फ़ाइलें जोड़ सकते हैं।
- **डायरेक्टरी स्कैन के विकल्प** में स्ट्रीमिंग आउटपुट, `--first`, `--limit`, सॉर्टिंग, वर्कर की संख्या चुनना, सिमलिंक का अनुसरण, Unix पर एक ही फ़ाइल सिस्टम तक स्कैन सीमित रखना और एक साथ खुली डायरेक्टरियों की सीमा तय करना शामिल है।
- **मेटाडेटा फ़िल्टर** फ़ाइल आकार की सटीक दशमलव या बाइनरी सीमाएँ और आखिरी बदलाव की समयावधि तय करते हैं। क्वेरी में ज़रूरत न हो, तो इंजन मेटाडेटा नहीं पढ़ता।
- **दोबारा इस्तेमाल की जाने वाली क्वेरी** एक छोटी टेक्स्ट कॉन्फ़िग फ़ाइल में नाम वाली प्रोजेक्ट क्वेरी प्रोफ़ाइल के रूप में सहेजी जाती हैं।
- **निदान के विकल्प** `--explain`, `--why PATH` और `--stats` क्रमशः योजना, किसी पाथ को शामिल करने या छोड़ने का कारण और स्कैन में मापा गया काम दिखाते हैं।
- **आउटपुट और वर्कफ़्लो** में सादे पाथ, वैकल्पिक मेटाडेटा के साथ JSON Lines, NUL से अलग किए गए पाथ, फ़ॉर्मैटिंग, रंग, हाइपरलिंक, शेल के बिना कमांड चलाना, सीमित आकार वाले बैच और शेल ऑटोकम्प्लीशन शामिल हैं।
- **लाइब्रेरी API** से एक `Query` को एक बार कंपाइल करें और CLI चलाए बिना Rust लाइब्रेरी के ज़रिए नतीजे पढ़ें।

सिंटैक्स, सीमाओं और लागू सुविधाओं की स्थिति के लिए [CLI संदर्भ](docs/reference/cli.md), [ग्लॉब सिंटैक्स](docs/reference/glob-syntax.md) और [सुविधाओं की स्थिति](docs/reference/feature-status.md) देखें।

## यह कैसे काम करता है

कई फ़िल्टर वाली खोज के एक सामान्य तरीके में पहले पूरा डायरेक्टरी ट्री स्कैन होता है, फिर ग्लॉब की जाँच होती है, बाहर रखने की शर्तें लागू होती हैं, मेटाडेटा पढ़ा जाता है और नतीजे इकट्ठे किए जाते हैं। Branchcut इन हिस्सों को एक क्वेरी योजना में कंपाइल करता है और ट्री में आगे बढ़ते हुए पैटर्न की वे अवस्थाएँ साथ रखता है जिनसे अभी मिलान संभव है।

```text
patterns + exclusions + ignores + filters + termination
                          │
                          ▼
             compiled traversal plan
       narrowed root · active pattern states
       pruning rules · metadata requirements
                          │
                          ▼
             stream matching filesystem paths
```

इस डिज़ाइन का उद्देश्य फ़ाइल सिस्टम पर गैरज़रूरी काम घटाना है। हर क्वेरी के लिए हर दूसरे टूल से तेज़ होने की गारंटी नहीं है। नीचे दिए गए तुलनात्मक माप और दर्ज की गई मापन विधि देखें।

## प्रदर्शन

तालिका में 18 नवंबर 2026 को Windows/NTFS पर कमिट किए गए खोज इम्प्लीमेंटेशन (`8035ad6`) के साथ लिए गए **डिफ़ॉल्ट CLI रन** के नतीजे हैं। हर टूल 100,000 फ़ाइलों वाले एक ही परीक्षण डेटा पर खोज करता है। समय मापने से पहले और बाद में नतीजों के सेट की जाँच की जाती है। हर मान तीन वॉर्मअप के बाद नई प्रोसेस में किए गए नौ रन की माध्यिका है। इसमें प्रोसेस शुरू करने और आउटपुट कैप्चर करने का समय शामिल है।

`n/t` का मतलब है कि बेंचमार्क रनर ने उस काम के लिए समकक्ष कमांड का समय नहीं मापा। इससे यह तय नहीं होता कि टूल में वह सुविधा मौजूद है या नहीं। ripgrep का समय `--files` मोड में लिया गया, जिसमें फ़ाइल की सामग्री नहीं खोजी जाती।

| वर्कलोड | Branchcut | zlob | ripgrep (`rg --files`) | fd | fast-glob | tinyglobby |
|---|---:|---:|---:|---:|---:|---:|
| बड़े दायरे में फ़ाइल खोज | **68.79 ms** | 123.94 ms | 109.17 ms | 117.14 ms | 370.45 ms | 242.30 ms |
| तय पाथ प्रीफ़िक्स वाली Rust फ़ाइलें | 40.34 ms | **35.46 ms** | 75.23 ms | 40.91 ms | 210.08 ms | 153.56 ms |
| कुछ पाथ छोड़कर Rust/TOML फ़ाइलें | **59.27 ms** | n/t | 79.27 ms | 92.57 ms | 333.23 ms | 208.11 ms |
| छिपे पाथ समेत Rust फ़ाइलें | **64.51 ms** | 98.29 ms | 84.53 ms | 94.45 ms | 314.34 ms | 195.53 ms |
| `.gitignore` के साथ Rust फ़ाइलें | **68.29 ms** | 80.07 ms | 77.18 ms | 113.49 ms | n/t | n/t |
| डायरेक्टरियाँ | **53.76 ms** | 80.35 ms | n/t | 84.66 ms | 251.20 ms | 152.86 ms |
| क्रमबद्ध Rust फ़ाइलें | **101.96 ms** | 103.99 ms | 260.04 ms | n/t | n/t | n/t |
| पहली 10 Rust फ़ाइलें | **37.50 ms** | 98.46 ms | n/t | 46.00 ms | n/t | n/t |
| बड़े दायरे की खोज, NUL आउटपुट | **72.61 ms** | n/t | 105.54 ms | 109.36 ms | n/t | n/t |
| सभी उपडायरेक्टरियों में केवल फ़ाइल नाम से Rust फ़ाइलें | **64.26 ms** | 96.60 ms | 76.10 ms | 94.15 ms | 318.07 ms | 208.26 ms |

तय पाथ प्रीफ़िक्स वाली डिफ़ॉल्ट क्वेरी में Zlob सबसे तेज़ है। उसी क्वेरी में चार वर्कर वाला Branchcut 33.39 ms लेता है। इस परीक्षण डेटा पर बाकी डिफ़ॉल्ट क्वेरियों में Branchcut आगे है। क्रमबद्ध नतीजों का समय zlob के करीब है। इन खास वर्कलोड पर लिए गए मापों से सभी परिस्थितियों की रैंकिंग तय नहीं होती। बड़े दायरे की फ़ाइल खोज में डिफ़ॉल्ट Branchcut का अधिकतम वर्किंग सेट 6.38 MiB दर्ज हुआ। हर टूल और जाँचे गए हर वर्कर कॉन्फ़िगरेशन के मेमोरी नमूने रखे गए हैं।

- [डिफ़ॉल्ट और ट्यून किए गए रन के पूरे नतीजे, मेमोरी और मापन विधि](docs/evidence/current-cli-comparison.md)
- [18 नवंबर के मूल नतीजों का रिकॉर्ड](benchmarks/results/2026-11-18-gap-closure-cli.json)
- [12 नवंबर की पुरानी तुलना](docs/evidence/cli-comparison.md)
- [बेंचमार्क की विधि और सीमाएँ](BENCHMARKS.md)

## Rust लाइब्रेरी

Rust लाइब्रेरी की मदद से होस्ट ऐप्लिकेशन एक क्वेरी को एक बार कंपाइल कर सकता है और मिलते हुए पाथ कॉलबैक को भेज सकता है।

```rust
use branchcut::{Completion, Query};
use std::{io, ops::ControlFlow, path::Path};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let query = Query {
        patterns: vec!["src/**/*.rs".into()],
        ..Query::default()
    }
    .compile()?;

    let report = query.visit(&mut |path: &Path| -> io::Result<ControlFlow<()>> {
        println!("{}", path.display());
        Ok(ControlFlow::Continue(()))
    })?;

    assert_eq!(report.completion, Completion::Complete);
    Ok(())
}
```

कॉलबैक `ControlFlow::Break` से डायरेक्टरी स्कैन रोक सकता है। त्रुटियों को सँभालने, सॉर्टिंग और विज़िटर के उदाहरणों के लिए [Rust लाइब्रेरी ट्यूटोरियल](docs/tutorials/rust-library.md) देखें।

## संगतता और सीमाएँ

Branchcut एक फ़ाइल सिस्टम क्वेरी टूल है जिसका दायरा सीमित है। fast-glob, tinyglobby, zlob, fd या ripgrep के पूरे API को सीधे Branchcut से बदला नहीं जा सकता। संगतता केवल दस्तावेज़ में बताए और जाँचे गए व्यवहार पर लागू होती है।

- अनदेखा करने के नियम विकल्प देने पर लागू होते हैं। `--gitignore` Git के इग्नोर स्रोतों और उपडायरेक्टरियों की `.gitignore` फ़ाइलों को सक्षम करता है। `--ignore` उपडायरेक्टरियों की `.ignore` फ़ाइलों को भी जोड़ता है। `--ignore-file` से सीधे दिए गए नियमों को सबसे ज़्यादा प्राथमिकता मिलती है। Git कॉन्फ़िगरेशन की कुछ शर्तें और इग्नोर सिंटैक्स के कुछ हिस्से अभी समर्थित नहीं हैं।
- डिफ़ॉल्ट मिलान बाइट के आधार पर होता है। वैध UTF-8 पाथ के लिए `--unicode` लगाने पर वाइल्डकार्ड Unicode स्केलर मानों पर काम करते हैं और अक्षरों का मिलान उन्हें साधारण रूप से लोअरकेस में बदलकर होता है। इसमें पूर्ण केस फ़ोल्डिंग, नॉर्मलाइज़ेशन या ग्रैफ़ीम मिलान उपलब्ध नहीं है।
- दशमलव में दी गई आकार की सीमाएँ सटीक होती हैं और उन्हें बाइट की पूर्ण संख्या के बराबर होना चाहिए। आखिरी बदलाव की तारीख के लिए UTC तारीखें, RFC3339 ऑफ़सेट और सेकंड के दशमलव अंश, तथा Unix `@seconds` समय-बिंदु समर्थित हैं।
- डायरेक्टरी सिमलिंक का अनुसरण तभी होता है जब उसका विकल्प दिया जाए। लिंक का अनुसरण करते समय स्कैन क्रम से चलता है और चक्रों की पहचान करता है। `--one-file-system` केवल Unix पर उपलब्ध है।
- `--max-open` में सीमित संख्या देने पर खुली डायरेक्टरियों के हैंडल की संख्या सीमित रहती है। लंबित एंट्री मेमोरी में रखी जाती हैं और स्कैन बाद में वहीं से जारी होता है। इससे कम हैंडल खुले रहते हैं और मेमोरी का इस्तेमाल बढ़ता है।
- Branchcut पाथ चुनता है और फ़ाइल सिस्टम का मेटाडेटा पढ़ता है। चुनी गई फ़ाइलों के अंदर शाब्दिक टेक्स्ट खोज `--content` (उपनाम `--grep`) से उपलब्ध है: कई पैटर्न OR से जुड़ते हैं, आउटपुट `पाथ:पंक्ति:स्तंभ:टेक्स्ट` होता है, `--text` के बिना बाइनरी फ़ाइलें छोड़ दी जाती हैं, और जिन क्वेरी में यह नहीं मांगा जाता वे सामग्री नहीं पढ़तीं। इसमें रेगुलर एक्सप्रेशन, कैप्चर या बहु-पंक्ति मिलान नहीं है। प्रोजेक्ट इंडेक्स भी नहीं बनाता, बदलावों की निगरानी नहीं करता और GUI उपलब्ध नहीं कराता।

- `--strict` के बिना फ़ाइल सिस्टम की त्रुटियों की सूचना दी जाती है और खोज जारी रहती है। सॉर्टिंग के लिए आउटपुट से पहले स्कैन पूरा होता है। सामान्य, बिना सॉर्टिंग वाली क्वेरी में नतीजे मिलते ही भेजे जाते हैं।

## दस्तावेज़

| यहाँ से शुरू करें | संदर्भ और उन्नत वर्कफ़्लो |
|---|---|
| [शुरू करने की गाइड](docs/getting-started/quick-start.md) | [CLI संदर्भ](docs/reference/cli.md) |
| [रोज़मर्रा की खोज के उदाहरण](docs/guides/everyday-search.md) | [ग्लॉब सिंटैक्स](docs/reference/glob-syntax.md) |
| [सहेजी गई क्वेरी](docs/guides/project-queries.md) | [अनदेखा करने के नियम और छिपे हुए पाथ](docs/guides/ignore-and-hidden.md) |
| [Rust लाइब्रेरी ट्यूटोरियल](docs/tutorials/rust-library.md) | [आउटपुट और कमांड चलाना](docs/guides/output-and-exec.md) |
| [सुविधाओं की स्थिति](docs/reference/feature-status.md) | [संगतता संदर्भ](docs/reference/compatibility.md) |

ऊपर दिए गए भाषा लिंक इस README के अनुवादों पर ले जाते हैं। विस्तृत दस्तावेज़ फ़िलहाल अंग्रेज़ी में हैं। [website/](website/README.md) में मौजूद साइट भी इन्हीं मूल दस्तावेज़ों से बनती है।

## योगदान

बग रिपोर्ट, किसी एक बदलाव पर केंद्रित पुल रिक्वेस्ट और अनुवाद में सुधार का स्वागत है। व्यवहार में हर पुष्ट रिग्रेशन के लिए टेस्ट ज़रूरी है। प्रदर्शन के दावों के लिए ऐसे माप चाहिए जिन्हें दोहराया जा सके। जिन मापों में Branchcut पीछे रहा है, उन्हें भी रखें।

```sh
cargo fmt -- --check
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
cargo tree -e normal
```

[योगदान की जाँच की गाइड](docs/contributing/verification.md), [आर्किटेक्चर के नियम](AGENTS.md), [रोडमैप](ROADMAP.md), [दस्तावेज़ लिखने की गाइड](docs/contributing/documentation.md) और [बेंचमार्क की मापन विधि](BENCHMARKS.md) पढ़ें। वेबसाइट उस Rust डिपेंडेंसी ग्राफ़ से अलग है जिसमें कोई बाहरी क्रेट नहीं है।

## शुरुआत और लाइसेंस

संस्करण टैग वाला [प्रतियोगिता सबमिशन](https://github.com/codex-mohan/branchcut/tree/submission-v1) ऐतिहासिक रिकॉर्ड के तौर पर मौजूद है। Branchcut पहली बार Zero Dependency 72-Hour Hackathon के लिए बनाया गया था। मौजूदा डेवलपमेंट प्रतियोगिता के बाद [AGENTS.md](AGENTS.md) में तय किए गए प्रोडक्ट और आर्किटेक्चर के नियमों के अनुसार चलता है। सबमिशन का टैग यथावत है।

Branchcut [MIT लाइसेंस](LICENSE) के तहत उपलब्ध है। डिपेंडेंसी का ब्यौरा [STDLIB.md](STDLIB.md) में देखें।
