# Assumptions and Shortcuts (Phase 3 Monitoring Mode)

Here is a summary of the assumptions, shortcuts, and unspecified decisions made during the Phase 3 implementation:

### 1. Ephemeral State for `PairStatus`
Because the instructions strictly stated, *"Do not modify `src/state/mod.rs`,"* I made the decision to treat `pair_statuses` as strictly ephemeral (UI-only) state. 
- **Assumption:** When the app cold-starts and loads existing `channel_pairs` from `state.json`, I initialize `pair_statuses` with `PairStatus::default()` (`Idle`) for every loaded pair. The app does not remember if a pair was previously in an `Error` state before being shut down.

### 2. Synchronization of `pair_statuses` length
The instructions noted to *"resize it whenever pairs are added or deleted,"* but didn't specify exactly where or how.
- **Decision:** I explicitly hooked into the `AppEvent::Input` key handlers for adding a pair (`a`) and deleting a pair (`d` -> `y`) to manually call `self.pair_statuses.push()` and `self.pair_statuses.remove()` at the exact moment `self.state.channel_pairs` is modified. 
- **Shortcut:** Instead of writing a helper method to synchronize the array lengths defensively on every tick, I surgically updated the lengths inline where mutations occur. 

### 3. Clearing Errors on Next Tick
The rules specified: *"Error is cleared back to Idle at the start of the next MonitoringTick (i.e. PairSyncStarted resets to Syncing)."*
- **Assumption:** I assumed that simply allowing `AppEvent::PairSyncStarted` to overwrite the `Error` status with `Syncing` at the start of the next run was sufficient to fulfill this requirement. I did not write explicit code to reset everything to `Idle` precisely on `AppEvent::MonitoringTick`, as the transition to `Syncing` naturally clears the visual error.

### 4. Layout Constraints and "Blank Line" Footer
The instructions said: *"If no error on selected pair, this line is blank. Add Constraint::Length(1) to the layout for this footer."*
- **Decision:** I added `Constraint::Length(1)` between the Table and the Help footer, changing the `chunks` array from 3 items to 4. 
- **Shortcut:** Instead of actively rendering an empty string (`Paragraph::new("")`) into the layout chunk when no error is present, I used an `if let` guard to simply skip rendering any widget into `chunks[2]`. Ratatui handles empty constraints cleanly by leaving the terminal background as is, fulfilling the "blank line" requirement natively. 

### 5. `AppEvent::PairError` payload matching
- **Shortcut:** In `src/app/mod.rs`, the `AppEvent::PairError` arm previously matched using catch-alls (`pair_index: _, error: _`). I replaced the catch-alls to capture the actual values and transition the status cleanly.

### 6. Clippy Refactoring (Collapsible `if`)
- **Decision:** While doing the final UI implementation for the error footer, I initially nested an `if` bounds check and an `if let` enum unwrap. `cargo clippy` flagged this as a `clippy::collapsible-if` warning. Rather than adding an `#[allow(...)]` attribute or forcing a messy boolean chain, I took the slightly cleaner shortcut of refactoring it to `if let Some(crate::app::PairStatus::Error(msg)) = app.pair_statuses.get(app.active_pair_index)` to safely combine bounds-checking and enum extraction into a single expression.
