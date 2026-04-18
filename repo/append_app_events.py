import sys

with open("src/app/mod.rs", "r") as f:
    content = f.read()

# Find the end of `match event { ... }` block. We know it ends with a bunch of `_ => {}` or similar in `handle_event`.
# Wait, actually we can just find where `AppEvent::PairError { ... } => {}` is, or `AppEvent::StartArchiveRun => { ... }` ends.
# Let's find the end of the `match event {` block.
