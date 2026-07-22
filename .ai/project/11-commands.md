# Команды проекта и гейты качества

Запуск из корня репозитория (Windows: PowerShell или Git Bash).

## Что работает сейчас (гринфилд)

До выполнения этапа S0 существует только синхронизация правил:

```powershell
python dev_tools_scripts_runner.py sync-agents           # пересобрать AGENTS.md
python dev_tools_scripts_runner.py sync-agents --check    # проверить актуальность
```

Это временный Python-скрипт с нулевой стоимостью запуска, нужный до появления
Rust-инструментария. Этап S0 переносит его в `cargo xtask sync-agents` и убирает
Python из репозитория.

## Целевые команды (появляются на S0)

```powershell
cargo xtask gate            # полный гейт качества — основной путь проверки
cargo xtask gate --quick    # быстрый гейт, минимум перед push
cargo xtask fmt             # форматирование Rust и фронтенда
cargo xtask doctor          # диагностика окружения без изменения состояния
cargo xtask sync-agents     # пересборка AGENTS.md
```

Прямые команды по слоям: `cargo fmt --all --check`,
`cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`,
`cargo deny check`, `cargo tauri dev` (после S11),
`pnpm --filter ui typecheck` (после S11).

## Политика гейтов

- Перед слиянием в `main` полный гейт обязан быть зелёным.
- Быстрый гейт — минимум для промежуточных пушей.
- `sync-agents --check` входит в гейт: рассинхрон `AGENTS.md` с `.ai/` намеренно
  ломает сборку.
- Изменения только в документах или конфигурации всё равно проходят проверки.
- CI гоняет матрицу `windows-latest` / `macos-latest` / `ubuntu-latest`:
  кроссплатформенность проверяется автоматически, а не на словах.

## Платформенные замечания

- Windows: длинные пути и антивирус мешают тестам с временными каталогами — использовать
  явный базовый временный каталог внутри репозитория.
- macOS: сборка Tauri требует Xcode Command Line Tools.
- Linux: Tauri требует `webkit2gtk` и связанные системные библиотеки.
