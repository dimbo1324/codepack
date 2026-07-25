# Текущее состояние архитектуры

> Этот документ описывает то, что **реально существует в коде**, а не то, что
> запланировано. План — в `ROADMAP.md`, замысел — в `BLUEPRINT.md`.
>
> Обновляется каждый раз, когда меняется форма системы: новый крейт, новый слой,
> новая операционная задача.

**Дата последней ревизии:** 2026-07-25
**Состояние:** этапы S0–S9 завершены (S6+S7 и S8+S9 — каждая пара выполнена в одной
ветке/задаче по явному указанию владельца). Все девять крейтов транша «Ядро»
(`codepack-core`, `-scanner`, `-security`, `-diff`, `-storage`, `-tokens`, `-reports`,
`-archive`, `-engine`) содержат реальную доменную логику — транш «Ядро» полностью
собран, работоспособного движка без интерфейса уже достаточно для headless-экспорта.
`codepack-cli` — единственный оставшийся плейсхолдер (этап S10).

2026-07-25 проведена **закалка ядра** (`ROADMAP.md` §8): паритет с legacy теперь
доказывается её реальным запуском (`tests/golden/` + `crates/codepack-engine/tests/golden.rs`
на трёх фикстурах), семь найденных расхождений исправлены, а три возможности, сданные в
S5/S6 без единого вызывающего (`fit_to_budget`, `ModelContextLimits`, `cleanup_old_runs`),
подключены к пайплайну.

## Что существует

| Область | Состояние |
|---|---|
| Cargo workspace (`Cargo.toml`, `resolver = "2"`) | **готов** |
| `codepack-core` | **готов (S1)**: `Config` (26 полей legacy + `schema_version`), нормализация, миграция legacy-настроек, 5 AI-пресетов (данные), `AppPaths`, `CancellationToken`, `ProgressEvent`/`LogEvent`, 5 общих типов пайплайна (`ExportPaths`, `CopyStats`, `TextDumpStats`, `RiskPreviewReport`, `ArchiveBuildResult`). **2026-07-25**: `profiles/` — второй legacy-файл настроек (`~/.project_exporter_profiles.json`, Q8): загрузка/сохранение, слияние с встроенными профилями, применение переопределений; `Config` получил `history_keep_last_n` и `token_budget`; `AppPaths` — `user_profiles_file()`/`model_limits_file()`. 72 юнит- + 6 интеграционных тестов |
| `codepack-scanner` | **готов (S2, в границах этапа — без safe-mode/diff-фильтрации, см. `ROADMAP.md` §2 Status)**: базовое+стековое игнорирование директорий (`walk.rs`, `IgnoredDirMatcher`, `walkdir` без следования симлинкам), `.exportignore`/кастомные правила (`ignore/`, ручной `fnmatch`-эквивалент на `regex`), детектор 12 стеков (`stack.rs`), классификация текст/бинарь (`classify.rs`), `build_export_plan()`/`write_export_plan_files()` (`plan/`, JSON+Markdown, порядок полей — контракт I5). **2026-07-25**: план применяет safe-export-mode через предикат вызывающей стороны (`SafetyClassifier`, без зависимости на `codepack-security`); `PlanSummary` вернул `estimated_included_size`/`skipped_dirs_count`; порядок обхода воспроизводит `os.walk` + NTFS-коллацию. 85 юнит- + 13 интеграционных тестов |
| `codepack-security` | **готов (S3)**: три safe-режима (`policy/`), редактирование секретов с безбэкреференсным переписыванием legacy-регекса (`redact.rs`), эвристический сканер v3 (`scan/`) — sensitive-файлы, secret-каскад (4 уровня уверенности), 9 risky-code правил, плюс новое из BLUEPRINT §B.1: 10 провайдер-сигнатур, энтропия Шеннона, `aho-corasick`-предфильтр (`patterns/`). Выходы `.txt`/`.json`/SARIF 2.1.0 (`scan/write/`). Корпус-baseline (I9): parity P=1.000/R=0.312/F1=0.476, full P=1.000/R=1.000/F1=1.000. 111 юнит- + 7 интеграционных тестов |
| `codepack-diff` | **готов (S4)**: 4 режима diff (`all`/`last_export`/`git_ref`/`uncommitted`) через `git2` (только чтение, никогда сеть; с 2026-07-25 `git_ref` сравнивает `base..target`, а не только `base..HEAD` — Q9), снапшот проекта с потоковым SHA-256 (`snapshot/`), `last_export` берёт предыдущий снапшот аргументом (без хранилища — это S5), отчёт `29_export_comparison_report.md`. Первая C-зависимость воркспейса (`libgit2-sys`, vendored, без `https`/`ssh`/`cred`-фич). 42 теста |
| `codepack-storage` | **готов (S5)**: SQLite-схема из 7 таблиц + `schema_version` (BLUEPRINT §D.2/§D.3), `rusqlite` (bundled, вторая C-зависимость воркспейса после `git2`), встроенные пронумерованные миграции, `record_export_run()` — единственная точка записи (снапшот только вставляется, никогда не обновляется — структурная гарантия инварианта I6), `import_legacy_history()` (явный opt-in, воспроизводит найденный legacy-баг с «отравленным» пустым снапшотом как есть), per-project ретеншн (`cleanup_old_runs`, `ON DELETE CASCADE`). Крейт не имеет ни одной рантайм-зависимости от `codepack-core` — принимает путь к БД параметром. 22 теста (включая WAL/конкурентный доступ на реальных файлах) |
| `codepack-tokens` | **готов (S6)**: `format_bytes` (порт 1:1, инвариант I4), `estimate_tokens_fallback` (`max(1, round(B/3.5))` — легаси использует `round`, не `ceil`, как упрощает BLUEPRINT §E.1) и калиброванный `estimate_tokens_refined` (ASCII/кириллица-UTF8), обе публичны и не подменяют друг друга. `ModelContextLimits` (4 записи legacy + `load_or_default` — слияние с файлом-переопределением, битый файл = ошибка; потребитель таблицы — S11, legacy использовал её только в GUI). `fit_to_budget` — детерминированный жадный отбор по плотности ценности, `importance` — параметр вызывающей стороны (движок передаёт ранжирование из `16_key_files_report`). Чистый крейт без зависимости от `codepack-core` или любого другого `codepack-*`. 22 теста |
| `codepack-reports` | **готов (S7)**: ~26 пронумерованных отчётов + `PROJECT_PROFILE.json`/`REPORT_PLUGINS.json`/`AI_CONTEXT/`/`AI_PROMPTS/`/`REPORT_DASHBOARD.html`/writer-функции `manifest.json`/`INDEX.md`. Плагинный раннер с гейтингом по 5 профилям, `catch_unwind`+`Result`-отказоустойчивостью, `ERROR_<имя>.txt`. `06_security_scan.*` — тонкая обёртка над `codepack-security`; `05_git_deep`/`21_git_timeline_report` — только `git2`, без подпроцессов; RU/EN-локализация — пилот на одном отчёте (Q12); известный, раскрытый пробел — проверка отмены только между отчётами, не внутри их циклов (Q13). 139 тестов |
| `codepack-archive` | **готов (S8)**: логическая группировка на 14 групп (`entry.rs`, точный порт приоритета legacy), `First-Fit`-планирование частей (`plan.rs`, цель 500 МБ / жёсткий лимит 512 МБ / резерв 8 МБ, крупный файл — своя часть), сборка ZIP уровня deflate 6 с post-write перепланированием в split при превышении лимита (`build.rs`, `on_plan_ready`-хук вместо зависимости от `codepack-reports`), `27_archive_plan.md/.json` (`report.rs`), восстановление с защитой от path-traversal — двойная проверка (`ZipFile::enclosed_name()` + собственная лексическая проверка компонентов пути), `ARCHIVE_SET_MANIFEST.json`/`RESTORE_INSTRUCTIONS.md` (`restore.rs`). Зависит только от `codepack-core`. 43 теста |
| `codepack-engine` | **готов (S9)**: восьмишаговый оркестратор пайплайна (BLUEPRINT §A.2) — план → копирование → структура → Git → текстовый дамп → аналитика → манифест → архив (`orchestrator::run_export` — единственная публичная точка входа для будущего вызывающего). Три новых, ранее не существовавших шага реализованы здесь: копирование (`copy.rs`, фильтрация off `ExportPlan.included_files`, без второго независимого обхода дерева), структура/Git/текстовый дамп (`structure.rs`, `git_report.rs` — только чтение через `git2`, `text_dump.rs` — 6-кодировочная цепочка фолбэка через `encoding_rs`). Аналитика (`analytics.rs`) — единственная точка вызова `codepack_security::scan_project` во всём пайплайне. Отмена проверяется внутри циклов каждого шага, а не только между шагами (риск Q13 закрыт на уровне пайплайна, но не внутри `codepack-reports`); шаги 7-8 и запись истории всегда выполняются, даже при отмене — так же, как в legacy. Успешность прогона — точный порт legacy `successful = !cancelled && !cancel.is_cancelled() && copy_stats.errors == 0`, гейтит запись нового снапшота-базлайна (I6) через `codepack-storage::record_export_run`. Очистка staging-каталога гарантирована на любом пути выхода (RAII `StagingCleanupGuard`, найдено и исправлено независимым ревью — раньше протекала при ошибках на середине пайплайна). Самый широкий граф зависимостей в воркспейсе (все домен-крейты + `git2` + `encoding_rs`), но не более широкий, чем нужно — UI/сеть не подключены. **2026-07-25**: `budget.rs` (подключение `fit_to_budget`), вызов ретеншна истории, реальный `redacted_count`, golden-тест против исполненного legacy на трёх фикстурах. 87 не-`#[ignore]` тестов (+ смок-тест на ≥50k файлов под `#[ignore]`) |
| `codepack-cli` | **плейсхолдер**: `main.rs`, заглушка стадии S10 |
| `cargo xtask` (`crates/xtask`) | **готов**: `gate`, `fmt`, `lint`, `test`, `deny`, `sync-agents [--check]`, `doctor` |
| `rust-toolchain.toml`, `rustfmt.toml`, workspace lints | **готовы** |
| `deny.toml` | **активен с S1**: advisories/bans/licenses/sources все `ok`; собственные крейты исключены из license-проверки (`[licenses.private] ignore = true`); с S2 добавлен `[bans] allow-wildcard-paths = true` (внутриворкспейсные path-зависимости без semver-диапазона — не supply-chain риск) |
| Десктоп-приложение (`apps/desktop`) | не создано — этап S11 |
| CI (`.github/workflows/ci.yml`) | **готов**: матрица `ubuntu-latest` / `macos-latest` / `windows-latest`, устанавливает `cargo-deny` и запускает `cargo xtask gate` |
| Golden-паритет (`tests/golden/`) | **готов (2026-07-25)**: эталоны реального вывода legacy на трёх фикстурах в репозитории, генератор `cargo xtask golden` (нужен Python только разработчику), сверка — `crates/codepack-engine/tests/golden.rs`. CI остаётся чисто Rust |
| Гейт качества (`cargo xtask gate`) | **зелёный локально и в CI**: fmt, clippy `-D warnings`, тесты, `cargo deny check`, `sync-agents --check` |
| Инфраструктура ИИ-агентов (`.ai/`, `.claude/`, `.codex/`) | **готова**; синхронизация `AGENTS.md` — `cargo xtask sync-agents` |
| Спецификация продукта (`BLUEPRINT.md`) | **готова** |
| План реализации (`ROADMAP.md`) | **готов** |
| Архив старой реализации | `docs/__arch__/codepack-main.zip` |

## Что было раньше

Предыдущая версия — Project Exporter Desktop 1.0.1: Python 3.11+, PySide6 (Qt),
около 13 400 строк, 22 тестовых модуля, только Windows, дистрибуция через PyInstaller
и Inno Setup. Полностью удалена из рабочего дерева; сохранена в архиве и подробно
описана в `BLUEPRINT.md` части A.

Причины переписывания: кроссплатформенность, производительность без ограничений GIL,
строгая типизация, замена плоских JSON на SQLite, усиление детектора секретов.

## Целевая форма системы

Описана в `BLUEPRINT.md` §C (архитектура Rust + Tauri) и §D (модель данных).
Порядок появления частей — в `ROADMAP.md` §1.

Кратко: слой крейтов `codepack-*` (ядро, независимое от UI) → `codepack-engine`
(оркестратор) → две точки входа: `codepack-cli` (headless) и `apps/desktop`
(Tauri + TypeScript).

## Известный незакрытый долг ядра

- **Q7** — `TEXT_EXTENSIONS`/`BINARY_EXTENSIONS`/`should_consider_text_file`/`looks_binary`
  по-прежнему продублированы в `codepack-scanner` и `codepack-security`. Владелец решил
  переносить их в `codepack-core` до S10; перенос не выполнен.
- **Q15** — провайдер-сигнатура «AWS Secret (по контексту)» (BLUEPRINT §B.1) не
  реализована: есть только Access Key ID (`AKIA…`).
- **Q13** — `codepack-reports` проверяет отмену только между отчётами, не внутри цикла по
  файлам каждого отчёта. На уровне пайплайна риск закрыт (движок проверяет отмену внутри
  циклов своих шагов), внутри крейта — нет.
- **Q12** — локализация артефактов остаётся пилотом на одном отчёте.
- **Q14** — разбиение архива использует First-Fit, а не First-Fit Decreasing.
- `cargo deny check` не запускался локально с S8: бинарь `cargo-deny` отсутствует в
  песочнице разработки. В CI он ставится и выполняется.

## Следующий шаг

Транш «Ядро» закрыт и закалён. Этап **S10 — CLI / headless** (`ROADMAP.md` §3, транш
«Интерфейс») — первая точка входа, которой реально может воспользоваться человек без
написания кода. Q6 (проектный конфиг `.codepack.toml`) решено делать там же.
