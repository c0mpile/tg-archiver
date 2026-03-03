---
description: Setup live monitoring of new messages to archive.
---

Using the tg-archiver project rules, implement the next phase of monitoring mode.

The phase is: Phase 1

Rules specific to monitoring mode:
- The ChannelPair struct is the unit of work — every archive operation takes a ChannelPair, not flat State fields.
- The polling loop must never run two archive workers for the same pair concurrently. Sequential processing per tick is required.
- poll_interval_secs must be clamped to a minimum of 60 before use — do not allow values below 60 even if the user sets them.
- AppEvent::PairSynced must trigger an atomic State::save() after updating the ChannelPair's last_forwarded_message_id.
- ActiveView::Monitoring is an addition to the existing view enum — it must not replace or interfere with the single-pair archive flow, which remains the setup path for adding a new ChannelPair.