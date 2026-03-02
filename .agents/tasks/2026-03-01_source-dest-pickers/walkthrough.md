# Source & Destination Pickers Implementation

## Changes Made
Replaced the text input fields in the "Select Source Channel" and "Select Destination Group" TUI views with browsable picker lists fetched directly from the Telegram API.

### 1. [TelegramClient](file:///home/kevin/dev/tg-archiver/repo/src/telegram/mod.rs#12-23) Additions
- Added `channel_list_cache` and `group_list_cache`.
- Implemented [get_joined_channels()](file:///home/kevin/dev/tg-archiver/repo/src/telegram/mod.rs#181-217) and [get_joined_groups()](file:///home/kevin/dev/tg-archiver/repo/src/telegram/mod.rs#218-252).
- Fetched `dialogs`, filtering them down via checking the newly inferred `grammers_client::peer::Peer` type and tracking their statuses through `.raw.broadcast` and `.raw.megagroup` fields.
- Both methods fully cache responses in-memory after their initial fetch calls to avoid redundant Telegram API queries, and handle rate-limits smoothly with the existing `retry_flood_wait!` wrapper.

### 2. App State Upgrades 
- Expanded the [App](file:///home/kevin/dev/tg-archiver/repo/src/app/mod.rs#106-124) struct with separate loaded lists `available_channels` and `available_groups`.
- Hooked `ListState` directly to the active components representing channels and groups.
- Discarded legacy `input_buffer` references to transition firmly to scrollable item selection. 
- Integrated a loading state when lists are being fetched.
- Updated `TopicSelect` to consistently utilize `ListState` matching the newly minted design.

### 3. TUI Makeover
- Retired `render_input` due to obsolescence.
- Established [render_channel_select](file:///home/kevin/dev/tg-archiver/repo/src/tui/mod.rs#56-106) and [render_group_select](file:///home/kevin/dev/tg-archiver/repo/src/tui/mod.rs#107-157). Both iterate over their respective lists utilizing `ListState` and formatting each option nicely as `[ID]  [Title]`.
- Implemented dynamic loading feedback for seamless transitions.

## Verification Results
Everything checks out cleanly:
- `cargo fmt -- --check`: Passed
- `cargo clippy -- -D warnings`: Passed (after fixing redundant braces, complexity, and collapsible structures)
- `cargo test`: Passed
- `cargo build --release`: Passed
