# Architecture & Implementation Decisions - Typed Auto-Topic Flag

This document summarizes the specific decisions, assumptions, and shortcuts taken during the refactor that were not explicitly detailed in the project rules or the user request.

## Decisions

### 1. State Reset on Topic Selection
- **Decision**: Explicitly set `state.auto_create_topic = false` whenever a user selects an existing topic from the list.
- **Rationale**: While the request focused on enabling the flag, ensuring it is cleared when moving away from auto-mode is critical for state consistency across restarts.

### 2. Topic Title Fallback
- **Decision**: Standardized the fallback topic title to `"Archive"` if `source_channel_title` is missing during automatic creation.
- **Rationale**: The previous magic-string logic had a similar fallback, but the refactor centralized it within the `StartArchiveRun` async task.

### 3. State field cleanup
- **Decision**: Chose to set `dest_topic_title` to `None` rather than an empty string when `auto_create_topic` is enabled.
- **Rationale**: `None` more accurately represents the absence of a chosen destination topic compared to an empty string, which might be interpreted as a valid title by some systems.

## Assumptions

### 1. Verification of Defaults in Tests
- **Assumption**: Added a specific check for `auto_create_topic == false` in the `test_migration_compatibility` test.
- **Rationale**: This verifies that the `#[serde(default)]` attribute correctly handles existing state files that lack the new field.

### 2. Immediate Atomic Save
- **Assumption**: Interpreted "save state atomically" as needing to happen immediately after the Telegram API returns the new topic ID, but before the main archive loop starts spawning download tasks.
- **Rationale**: Ensures that if the app crashes during the initial message scan, the newly created topic ID is already persisted.

## Shortcuts

### 1. Test Restoration
- **Correction/Shortcut**: During the first edit to `src/state/mod.rs`, a line in the migration test was accidentally deleted due to a targeting error in `multi_replace_file_content`. I restored this line in a subsequent step rather than performing a full git rollback, as the state was otherwise correct.

### 2. Verification Command
- **Shortcut**: Used `rg` with an exit code check (`1` for no matches) to verify removals. This is faster than a full codebase scan but assumes the string literal is the only way that pattern would exist.
