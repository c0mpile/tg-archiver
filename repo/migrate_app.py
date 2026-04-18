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

# Replace 's' command check
code = code.replace(
    "if self.state.source_channel_id.is_none() {",
    "if self.state.channel_pairs[self.active_pair_index].source_channel_id == 0 {"
).replace(
    "if self.state.dest_group_id.is_none() {",
    "if self.state.channel_pairs[self.active_pair_index].dest_group_id == 0 {"
).replace(
    "if self.state.dest_topic_id.is_none()",
    "if self.state.channel_pairs[self.active_pair_index].dest_topic_id.is_none()"
)

# Replace channel/group selection Enters
code = code.replace(
    "self.state.source_channel_id = Some(*id);",
    "self.state.channel_pairs[self.active_pair_index].source_channel_id = *id;"
).replace(
    "self.state.source_channel_title = Some(title.clone());",
    "self.state.channel_pairs[self.active_pair_index].source_channel_title = title.clone();"
).replace(
    "self.state.dest_group_id = Some(*id);",
    "self.state.channel_pairs[self.active_pair_index].dest_group_id = *id;"
).replace(
    "self.state.dest_group_title = Some(title.clone());",
    "self.state.channel_pairs[self.active_pair_index].dest_group_title = title.clone();"
).replace(
    "self.state.dest_topic_id = None;",
    "self.state.channel_pairs[self.active_pair_index].dest_topic_id = None;"
).replace(
    "self.state.dest_topic_title = None;",
    "self.state.channel_pairs[self.active_pair_index].dest_topic_title = None;"
).replace(
    "self.state.dest_topic_id = Some(*id);",
    "self.state.channel_pairs[self.active_pair_index].dest_topic_id = Some(*id);"
).replace(
    "self.state.dest_topic_title = Some(title.clone());",
    "self.state.channel_pairs[self.active_pair_index].dest_topic_title = Some(title.clone());"
)

# AppEvent::StartArchiveRun
old_start = """            AppEvent::StartArchiveRun => {
                self.active_view = ActiveView::ArchiveProgress;

                let mut state_clone = self.state.clone();
                let tg_clone = Arc::clone(telegram);
                let tx_clone = tx.clone();
                let paused_clone = Arc::clone(&self.is_paused);

                if let Some(source_id) = self.state.source_channel_id {
                    let dest_id = self.state.dest_group_id;
                    tokio::spawn(async move {
                        let source_missing = tg_clone.get_input_peer(source_id).await.is_none();
                        let dest_missing = match dest_id {
                            Some(id) => tg_clone.get_input_peer(id).await.is_none(),
                            None => false,
                        };

                        if source_missing || (dest_id.is_some() && dest_missing) {
                            let _ = tg_clone.get_joined_channels().await;
                            let _ = tg_clone.get_joined_groups().await;
                        }

                        // Handle automatic topic creation
                        if state_clone.auto_create_topic
                            && let Some(group_id) = state_clone.dest_group_id
                        {
                            let topic_title = state_clone
                                .source_channel_title
                                .as_deref()
                                .unwrap_or("Archive");
                            match tg_clone.create_topic(group_id, topic_title).await {
                                Ok(new_topic_id) => {
                                    state_clone.dest_topic_id = Some(new_topic_id);
                                    state_clone.dest_topic_title = Some(topic_title.to_string());
                                    state_clone.auto_create_topic = false;
                                    let s_clone = state_clone.clone();
                                    let _ = s_clone.save().await;
                                }
                                Err(e) => {
                                    let _ = tx_clone.try_send(AppEvent::ArchiveError(format!(
                                        "Failed to create topic: {}",
                                        e
                                    )));
                                    return;
                                }
                            }
                        }

                        crate::archive::start_archive_run(
                            state_clone,
                            tg_clone,
                            tx_clone,
                            paused_clone,
                        );
                    });
                }
            }"""

new_start = """            AppEvent::StartArchiveRun => {
                self.active_view = ActiveView::ArchiveProgress;

                let mut state_clone = self.state.clone();
                let tg_clone = Arc::clone(telegram);
                let tx_clone = tx.clone();
                let paused_clone = Arc::clone(&self.is_paused);
                let active_idx = self.active_pair_index;

                if self.state.channel_pairs[self.active_pair_index].source_channel_id != 0 {
                    let source_id = self.state.channel_pairs[self.active_pair_index].source_channel_id;
                    let dest_id = self.state.channel_pairs[self.active_pair_index].dest_group_id;
                    let dest_id_opt = if dest_id == 0 { None } else { Some(dest_id) };
                    tokio::spawn(async move {
                        let source_missing = tg_clone.get_input_peer(source_id).await.is_none();
                        let dest_missing = match dest_id_opt {
                            Some(id) => tg_clone.get_input_peer(id).await.is_none(),
                            None => false,
                        };

                        if source_missing || (dest_id_opt.is_some() && dest_missing) {
                            let _ = tg_clone.get_joined_channels().await;
                            let _ = tg_clone.get_joined_groups().await;
                        }

                        // Handle automatic topic creation
                        if state_clone.auto_create_topic
                            && state_clone.channel_pairs[active_idx].dest_group_id != 0
                        {
                            let group_id = state_clone.channel_pairs[active_idx].dest_group_id;
                            let topic_title = state_clone.channel_pairs[active_idx].source_channel_title.clone();
                            let topic_title_str = if topic_title.is_empty() { "Archive" } else { &topic_title };
                            match tg_clone.create_topic(group_id, topic_title_str).await {
                                Ok(new_topic_id) => {
                                    state_clone.channel_pairs[active_idx].dest_topic_id = Some(new_topic_id);
                                    state_clone.channel_pairs[active_idx].dest_topic_title = Some(topic_title_str.to_string());
                                    state_clone.auto_create_topic = false;
                                    let s_clone = state_clone.clone();
                                    let _ = s_clone.save().await;
                                }
                                Err(e) => {
                                    let _ = tx_clone.try_send(AppEvent::ArchiveError(format!(
                                        "Failed to create topic: {}",
                                        e
                                    )));
                                    return;
                                }
                            }
                        }

                        crate::archive::start_archive_run(
                            state_clone,
                            active_idx,
                            tg_clone,
                            tx_clone,
                            paused_clone,
                        );
                    });
                }
            }"""

code = code.replace(old_start, new_start)

# Save Cursor
code = code.replace(
    "self.state.last_forwarded_message_id = Some(cursor);",
    "self.state.channel_pairs[self.active_pair_index].last_forwarded_message_id = Some(cursor);"
)

# PromptResumeResult
old_resume = """            AppEvent::PromptResumeResult(resume) => {
                if resume {
                    self.active_view = ActiveView::ArchiveProgress;
                    let state_clone = self.state.clone();
                    let tg_clone = Arc::clone(telegram);
                    let tx_clone = tx.clone();
                    let paused_clone = Arc::clone(&self.is_paused);

                    if let Some(source_id) = self.state.source_channel_id {
                        let dest_id = self.state.dest_group_id;
                        tokio::spawn(async move {
                            let source_missing = tg_clone.get_input_peer(source_id).await.is_none();
                            let dest_missing = match dest_id {
                                Some(id) => tg_clone.get_input_peer(id).await.is_none(),
                                None => false,
                            };

                            if source_missing || (dest_id.is_some() && dest_missing) {
                                let _ = tg_clone.get_joined_channels().await;
                                let _ = tg_clone.get_joined_groups().await;
                            }
                            crate::archive::start_archive_run(
                                state_clone,
                                tg_clone,
                                tx_clone,
                                paused_clone,
                            );
                        });
                    }
                } else {
                    self.state.last_forwarded_message_id = None;
                    self.state.source_message_count = None;
                    let state_clone = self.state.clone();"""

new_resume = """            AppEvent::PromptResumeResult(resume) => {
                if resume {
                    self.active_view = ActiveView::ArchiveProgress;
                    let state_clone = self.state.clone();
                    let tg_clone = Arc::clone(telegram);
                    let tx_clone = tx.clone();
                    let paused_clone = Arc::clone(&self.is_paused);
                    let active_idx = self.active_pair_index;

                    if self.state.channel_pairs[self.active_pair_index].source_channel_id != 0 {
                        let source_id = self.state.channel_pairs[self.active_pair_index].source_channel_id;
                        let dest_id = self.state.channel_pairs[self.active_pair_index].dest_group_id;
                        let dest_id_opt = if dest_id == 0 { None } else { Some(dest_id) };
                        tokio::spawn(async move {
                            let source_missing = tg_clone.get_input_peer(source_id).await.is_none();
                            let dest_missing = match dest_id_opt {
                                Some(id) => tg_clone.get_input_peer(id).await.is_none(),
                                None => false,
                            };

                            if source_missing || (dest_id_opt.is_some() && dest_missing) {
                                let _ = tg_clone.get_joined_channels().await;
                                let _ = tg_clone.get_joined_groups().await;
                            }
                            crate::archive::start_archive_run(
                                state_clone,
                                active_idx,
                                tg_clone,
                                tx_clone,
                                paused_clone,
                            );
                        });
                    }
                } else {
                    self.state.channel_pairs[self.active_pair_index].last_forwarded_message_id = None;
                    let state_clone = self.state.clone();"""

code = code.replace(old_resume, new_resume)


with open("src/app/mod.rs", "w") as f:
    f.write(code)
