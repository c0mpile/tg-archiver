import re

with open("src/app/mod.rs", "r") as f:
    code = f.read()

# 1. Update App struct
code = re.sub(
    r"pub is_paused: Arc<std::sync::atomic::AtomicBool>,",
    "pub is_paused: Arc<std::sync::atomic::AtomicBool>,\n    pub active_pair_index: usize,\n    pub source_message_count: Option<i32>,",
    code
)

# 2. Update App::new signature and has_partial_state
code = re.sub(
    r"pub fn new\(config: Config, state: State\) -> Self \{\n        let has_partial_state =\n            state\.last_forwarded_message_id\.is_some\(\) && state\.source_message_count\.is_some\(\);",
    r"""pub fn new(config: Config, mut state: State) -> Self {
        if state.channel_pairs.is_empty() {
            state.channel_pairs.push(crate::state::ChannelPair::default());
        }
        let has_partial_state =
            state.channel_pairs[0].last_forwarded_message_id.is_some();""",
    code
)

# 3. Update Self initialization
code = re.sub(
    r"is_paused: Arc::new\(std::sync::atomic::AtomicBool::new\(false\)\),\n        \}",
    r"""is_paused: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            active_pair_index: 0,
            source_message_count: None,
        }""",
    code
)

# 4. Replace self.state.[field]
for field in [
    "source_channel_id",
    "source_channel_title",
    "dest_group_id",
    "dest_group_title",
    "dest_topic_id",
    "dest_topic_title",
    "last_forwarded_message_id",
]:
    code = re.sub(
        rf"self\.state\.{field}\b",
        f"self.state.channel_pairs[self.active_pair_index].{field}",
        code
    )

# 5. Fix StartArchiveRun and PromptResumeResult
# In StartArchiveRun:
code = re.sub(
    r"let mut state_clone = self\.state\.clone\(\);\n                let tg_clone = Arc::clone\(&telegram\);\n                let tx_clone = tx\.clone\(\);\n                let paused_clone = Arc::clone\(&self\.is_paused\);",
    r"""let mut state_clone = self.state.clone();
                let tg_clone = Arc::clone(&telegram);
                let tx_clone = tx.clone();
                let paused_clone = Arc::clone(&self.is_paused);
                let active_idx = self.active_pair_index;""",
    code
)

# In PromptResumeResult:
code = re.sub(
    r"let state_clone = self\.state\.clone\(\);\n                    let tg_clone = Arc::clone\(&telegram\);\n                    let tx_clone = tx\.clone\(\);\n                    let paused_clone = Arc::clone\(&self\.is_paused\);",
    r"""let state_clone = self.state.clone();
                    let tg_clone = Arc::clone(&telegram);
                    let tx_clone = tx.clone();
                    let paused_clone = Arc::clone(&self.is_paused);
                    let active_idx = self.active_pair_index;""",
    code
)


# In StartArchiveRun and PromptResumeResult, state_clone.[field] is used
for field in [
    "source_channel_id",
    "source_channel_title",
    "dest_group_id",
    "dest_group_title",
    "dest_topic_id",
    "dest_topic_title",
]:
    code = re.sub(
        rf"state_clone\.{field}\b",
        f"state_clone.channel_pairs[active_idx].{field}",
        code
    )

# 6. StartArchiveRun function call signatures
# start_archive_run(state_clone, tg_clone, tx_clone, paused_clone);
code = re.sub(
    r"crate::archive::start_archive_run\(\n\s*state_clone,\n\s*tg_clone,\n\s*tx_clone,\n\s*paused_clone,\n\s*\);",
    r"""crate::archive::start_archive_run(
                                state_clone,
                                active_idx,
                                tg_clone,
                                tx_clone,
                                paused_clone,
                            );""",
    code
)

# Remove source_message_count from Resume prompt false branch
code = re.sub(
    r"self\.state\.source_message_count = None;\n\s*",
    "",
    code
)


# Handle some specific field writes that shouldn't be Option unwrapped wrongly
# Oh wait, source_channel_id is NOT an Option in ChannelPair, it's i64!
# The flat fields were Option<i64>.
# Wait, in ChannelPair:
# pub source_channel_id: i64,
# pub dest_group_id: i64,
# pub dest_topic_id: Option<i32>,
#
# But wait, in the old State:
# pub source_channel_id: Option<i64>,
#
# So `if let Some(source_id) = self.state.channel_pairs[self.active_pair_index].source_channel_id` will fail compilation because it's an i64, not Option<i64>!
# Same for `if self.state.channel_pairs[self.active_pair_index].source_channel_id.is_none()`!
pass

