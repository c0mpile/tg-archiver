# Assumptions and Shortcuts — Topic ID Persistence Bug Fix

## Assumptions
1. **Auto-creation focus**: I assumed the user was primarily experiencing the bug with auto-created topics, as this was the most clear path to state desynchronization (where the main thread's `self.state` was never updated with the new ID).
2. **Single Pair Usage**: I assumed the user is currently only using the first channel pair (index 0), as `active_pair_index` is hardcoded to 0 in `App::new`. I maintained this pattern for consistency while fixing the bug.
3. **Serde Default Behavior**: I verified via a reproduction test that `Option` fields are handled correctly by `serde` when missing, but I assumed it was still safer to explicitly add `#[serde(default)]` to all optional fields to prevent future regressions during schema changes.

## Shortcuts
1. **Direct Event Handling**: Instead of a complex state synchronization primitive (like a shared `Arc<RwLock<State>>`), I used the existing `AppEvent` system to notify the main thread. This is a shortcut that avoids significant refactoring but remains within the "Surgical Modification" rule.
2. **Simplified Sync**: I updated the state and triggered a save immediately in the `TopicCreated` handler. A more optimized approach might have batched this, but given the frequency of topic creation (once per run), this is acceptable and safer.
3. **Task Folder Catch-up**: I created the task audit trail folder and files at the end of the task due to the immediate start of investigation.
