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
- Input Action - includes config parsing

## Customizable Parts

- Events
  - Broadcast Events
  - `UpdateContext` - additional "global" state
- Actions
  - Input actions
  - Common navigation actions should be provided?
- Components
  - Provide default components like Select
