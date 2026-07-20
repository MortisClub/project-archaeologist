<p align="center">
  <img src="assets/icon.png" width="128" alt="Project Archaeologist">
</p>

<h1 align="center">Project Archaeologist</h1>

<p align="center">
  Понять любой проект за минуту — локально, без сервера и облака.<br>
  Стек, структура, мёртвый код, важные файлы, устаревшие зависимости и история git.
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Rust-CLI-d9853b?style=flat-square&logo=rust&logoColor=white" alt="Rust">
  <img src="https://img.shields.io/badge/license-MIT-2b1a10?style=flat-square" alt="MIT">
  <img src="https://img.shields.io/badge/%C2%A9%20MortisClub-2026-2b1a10?style=flat-square" alt="MortisClub">
</p>

---

Открываешь старый проект, о котором уже ничего не помнишь, — и за минуту понимаешь, что это.
Одна команда печатает короткую сводку и кладёт рядом полный отчёт: `archaeology-report.json`
и самодостаточную карту `archaeology-report.html`, которую можно открыть в браузере.

<p align="center">
  <img src="assets/report-overview.png" width="820" alt="HTML-отчёт">
</p>

## Что оно находит

За один проход:

- стек и фреймворки — из манифестов (`Cargo.toml`, `package.json`, `go.mod`, `Dockerfile`, …);
- языки — разбивка по числу файлов и размеру;
- неиспользуемые файлы — исходники, которые никто не импортирует;
- самые важные файлы — по числу входящих импортов, частоте правок в git и размеру;
- зависимости, а с `--check-updates` — и какие из них отстали от реестра;
- дубликаты — по хэшу содержимого;
- карту директорий по объёму и самые большие файлы;
- историю git — коммиты, возраст, что менялось чаще всего, куда проект рос первым.

«Неиспользуемые файлы» — это не грубая эвристика: обходится реальный граф импортов, а точки
входа (`main`, `index`, `__init__.py`, …) и тесты из списка исключаются, так что остаётся
действительно мёртвый код. Граф строится для JavaScript/TypeScript и Python; на других языках
файлы попадают в инвентарь, но без графа — и отчёт про это честно пишет.

<p align="center">
  <img src="assets/report-unused.png" width="820" alt="Неиспользуемые файлы и устаревшие зависимости">
</p>

Историю проекта я беру из git, а не из файловых времён: те слишком часто врут после
копирования, распаковки архива или свежего checkout. Поэтому без репозитория инструмент
работает, но раздел истории просто пропускается.

## Установка

Нужен [Rust](https://rustup.rs).

```
git clone https://github.com/MortisClub/project-archaeologist
cd project-archaeologist
cargo build --release
```

Бинарник — `target/release/archaeologist`.

## Использование

```
archaeologist scan <путь>
```

- `--out <dir>` — куда положить отчёт (по умолчанию текущая папка)
- `--open` — открыть HTML-отчёт по завершении
- `--check-updates` — сверить зависимости с npm, crates.io и PyPI (нужна сеть)

```
archaeologist scan ../some-old-project --check-updates --open
```

## Как устроено

- `src/scan.rs` — обход дерева с учётом `.gitignore`
- `src/stack.rs` — языки, маркеры стека, фреймворки
- `src/imports.rs` — граф импортов, мёртвый код, важность файлов
- `src/deps.rs` — инвентарь зависимостей и проверка устаревших
- `src/dupes.rs` — дубликаты по SHA-256
- `src/git.rs` — история из `git log`
- `src/report.rs`, `templates/report.html` — JSON и интерактивная карта
- `tools/make-icons.js` — генерит иконку в `assets/`

## Лицензия

MIT, см. [LICENSE](LICENSE).

<p align="center">© MortisClub 2026</p>
