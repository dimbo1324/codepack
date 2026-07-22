<!-- tier: extended -->

# Легаси-эталон: старая реализация на Python

> **Суть.** `docs/__arch__/codepack-main.zip` — эталон поведения, а не кода; распаковывать только во временный каталог вне репозитория; брать оттуда факты и форматы, не архитектуру.

Предыдущая версия продукта (Project Exporter Desktop 1.0.1, Python + PySide6, только
Windows) сохранена как архив:

```text
docs/__arch__/codepack-main.zip
```

Это **эталон поведения**, а не образец кода. Новая реализация обязана воспроизвести
её результаты, но не её структуру.

## Когда обращаться к архиву

- Нужны точные значения констант: наборы расширений, чувствительные имена и суффиксы,
  списки игнорируемых каталогов, правила режимов безопасности.
- Нужен точный формат артефакта: `manifest.json`, `PROJECT_PROFILE.json`, SARIF,
  имена и порядок секций отчётов.
- Поведение неочевидно и `BLUEPRINT.md` не отвечает на вопрос однозначно.
- Нужен golden-эталон для теста паритета.

## Порядок работы с архивом

1. Сначала искать ответ в `BLUEPRINT.md` — там описана вся логика старой версии.
   Архив нужен, когда требуется буквальная точность.
2. Распаковывать **во временный каталог вне репозитория**. Никогда не распаковывать
   в рабочее дерево и никогда не коммитить распакованное содержимое.
3. Брать из архива только факты: значения, форматы, порядок шагов.
4. Не копировать архитектурные решения Python-версии в Rust: слои, имена модулей и
   способ организации кода задаются `ROADMAP.md` и модулем доменных правил.

## Соответствие старых модулей новым крейтам

| Старый модуль (Python) | Новый крейт |
|---|---|
| `services/exporter.py` | `codepack-engine` |
| `services/copy_service.py`, `export_plan.py` | `codepack-scanner` |
| `services/export_policy.py`, `export_ignore.py` | `codepack-scanner` + `codepack-security` |
| `utils/text_utils.py`, `reports/insights/security.py` | `codepack-security` |
| `services/stack_detector.py` | `codepack-scanner` |
| `services/diff_service.py`, `incremental.py` | `codepack-diff` |
| `services/archive_service.py` | `codepack-archive` |
| `services/export_history.py` | `codepack-storage` |
| `utils/token_counter.py` | `codepack-tokens` |
| `reports/**` | `codepack-reports` |
| `config.py`, `constants.py`, `models.py` | `codepack-core` |
| `gui/**` (PySide6) | `apps/desktop` (Tauri) |

## Чего в архиве брать нельзя

- Русскоязычные строки прогресса и отчётов копируются как есть только там, где это
  контракт формата; интерфейсные строки проходят через новую систему локализации.
- Известные слабости старой версии не переносятся: keyword-детект секретов усиливается
  (этап S3), плоские JSON заменяются на SQLite (S5), Windows-специфика убирается.
