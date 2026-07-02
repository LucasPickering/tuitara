# tuitara

[![Test CI](https://github.com/github/docs/actions/workflows/test.yml/badge.svg)](https://github.com/LucasPickering/tuitara/actions)
[![crates.io](https://img.shields.io/crates/v/tuitara.svg)](https://crates.io/crates/tuitara)
[![docs.rs](https://img.shields.io/docsrs/tuitara)](https://docs.rs/tuitara)

## Provided

- Main loop
- Component
  - Event handling
  - Drawing - Canvas
  - Theme/styling
  - State persistence
- Input
  - Key bindings - key codes and modifiers
  - Action list is NOT provided - app has to map inputs to actions themself

## What needs to be parameterized?

- Events
  - Broadcast Events
  - `UpdateContext` - additional "global" state
  - `Action` type on `InputEvent`
- Input
  - `Action` type
- Components
  - `UpdateContext`
  - `PersistentStore`
  - `Action`

## Provided Components

- Select
- FixedSelect
- Button
- TextBox
- Table
