---
name: legacy-lookup
description: Use when you need the exact behavior, constants, or artifact format of the old Python implementation stored in docs/__arch__/codepack-main.zip.
---

# Обращение к старой реализации

Старая версия (Project Exporter Desktop 1.0.1, Python + PySide6) сохранена как архив:

```text
docs/__arch__/codepack-main.zip
```

Это **эталон поведения, а не образец кода**.

## Сначала — BLUEPRINT

`BLUEPRINT.md` описывает всю логику старой версии: конвейер из 8 шагов, все 25 полей
конфигурации, режимы безопасности с наборами суффиксов, детектор стека, каталог из
~30 отчётов, параметры архивации, формулы. **В большинстве случаев ответ там.**

Архив нужен, только когда требуется буквальная точность: полный список констант,
дословный формат артефакта, порядок секций в отчёте.

## Порядок работы с архивом

1. Распаковывайте **во временный каталог вне репозитория**:

   ```powershell
   $tmp = Join-Path $env:TEMP "codepack-legacy"
   Expand-Archive -Path docs\__arch__\codepack-main.zip -DestinationPath $tmp -Force
   ```

2. Никогда не распаковывайте в рабочее дерево и не коммитьте распакованное содержимое.
3. Берите из архива только факты: значения констант, форматы, порядок шагов.
4. Не переносите архитектуру Python-версии в Rust: слои и организация кода задаются
   `ROADMAP.md` и `.ai/project/12-domain-rules.md`.

## Где что лежало в старой версии

| Что ищете | Файл в архиве |
|---|---|
| Константы, наборы расширений, чувствительные имена | `src/project_exporter_desktop/constants.py` |
| Поля конфигурации и их нормализация | `config.py` |
| Конвейер экспорта | `services/exporter.py` |
| Режимы безопасности | `services/export_policy.py` |
| Редактирование секретов | `utils/text_utils.py` |
| Сканер безопасности | `reports/insights/security.py` |
| Детектор стека | `services/stack_detector.py` |
| Дифференциальный экспорт | `services/diff_service.py` |
| Архивация и разбиение | `services/archive_service.py` |
| Каталог отчётов | `reports/insights/orchestrator.py` |
| Оценка токенов | `utils/token_counter.py` |

## Что не переносить

- Известные слабости: keyword-детект секретов усиливается (S3), плоские JSON заменяются
  на SQLite (S5), Windows-специфика убирается.
- Русскоязычные строки интерфейса — они проходят через новую систему локализации.
  Дословно сохраняются только строки, являющиеся частью контракта формата.
