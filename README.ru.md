# PicoOS

PicoOS — это небольшой экспериментальный kernel операционной системы, написанный на Rust для **RISC-V 64-bit**.

Проект создается как учебная и исследовательская ОС: шаг за шагом реализуются базовые механизмы ядра — загрузка, UART-вывод, trap/exception handling, timer interrupts, разметка памяти, page allocator, heap, таблица задач, стеки задач, cooperative task switching и resume задач после `yield`.

Сейчас PicoOS рассчитан на запуск в **QEMU RISC-V virt**.

## Статус проекта

PicoOS теперь развивается как **RISC-V-first** проект.

Ранее в проекте была экспериментальная ARM64/multiarch-поддержка, но она была удалена из основной версии, чтобы сделать код меньше, чище и проще для дальнейшего развития. ARM64/multiarch-версия сохранена отдельно как legacy-история.

Текущий рабочий milestone:

- загрузка RISC-V kernel в QEMU
- UART console output
- trap/exception handling
- timer interrupt support
- page allocator
- простой kernel heap
- task table
- отдельные стеки задач
- cooperative `yield`
- resume задачи после yield
- repeated yield/resume test
- scheduler-oriented resume loop test

## Архитектура

Текущая целевая архитектура:

```text
riscv64gc-unknown-none-elf
```

Проект настроен так, что RISC-V target используется по умолчанию. Поэтому в большинстве команд больше не нужно явно указывать `--target`.

## Требования

Нужно установить:

- Rust nightly или Rust toolchain, подходящий для `no_std` bare-metal разработки
- RISC-V target:

```bash
rustup target add riscv64gc-unknown-none-elf
```

- QEMU с поддержкой RISC-V:

```bash
qemu-system-riscv64
```

## Сборка

```bash
cargo build
```

Благодаря `.cargo/config.toml` проект по умолчанию собирается под:

```text
riscv64gc-unknown-none-elf
```

## Запуск в QEMU

Типичная команда запуска:

```bash
qemu-system-riscv64 \
  -machine virt \
  -nographic \
  -bios none \
  -kernel target/riscv64gc-unknown-none-elf/debug/PicoOS
```

Для конкретных тестовых сценариев лучше использовать скрипты из папки:

```text
scripts/
```

## Проверки

Запуск всех текущих проверок:

```bash
./scripts/check-all.sh
```

Сейчас проверяется RISC-V-сборка и selftest-конфигурации.

## Feature flags

В PicoOS используются feature flags для экспериментальных kernel-тестов.

Основные features:

```text
selftest
task_bootstrap_test
task_stack_switch_test
sequential_task_test
task_yield_test
scheduler_skip_finished_test
scheduler_driven_task_test
resume_candidate_test
resume_preflight_test
resume_dry_run_test
resume_restore_test
real_resume_restore_test
real_resume_restore_jump
two_yield_task_test
scheduler_resume_loop_test
verbose_resume_debug
```

Пример сборки с тестовыми features:

```bash
cargo build --features "task_yield_test resume_candidate_test resume_preflight_test resume_dry_run_test resume_restore_test"
```

## Текущий milestone: cooperative task resume

Главный текущий результат — рабочий RISC-V cooperative task resume flow:

```text
task -> yield -> kernel -> restore -> task -> yield -> kernel -> restore -> task -> exit -> kernel
```

Это подтверждает, что PicoOS уже умеет:

- запускать задачу на собственном стеке
- сохранять task context при yield
- возвращаться в kernel
- восстанавливать задачу
- продолжать выполнение после yield
- снова делать yield
- снова восстанавливаться
- корректно завершать задачу через task exit
- помечать задачу как finished

## Направление развития

Дальше PicoOS будет развиваться вокруг RISC-V:

- более чистый cooperative scheduler
- улучшенные переходы task state
- эксперименты с timer-driven scheduling
- syscall layer
- эксперименты с user mode
- memory protection
- позже — filesystem и storage experiments

Цель проекта — не сразу создать production-ready OS, а построить небольшое понятное ядро, где каждый механизм реализуется и проверяется по шагам.

## Лицензия

Лицензия пока не определена.
