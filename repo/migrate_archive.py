import re

with open("src/archive/mod.rs", "r") as f:
    code = f.read()

# start_archive_run
old_start_archive_run = """pub fn start_archive_run(
    state: State,
    telegram_client: Arc<TelegramClient>,
    tx: mpsc::Sender<AppEvent>,
    pause_flag: Arc<std::sync::atomic::AtomicBool>,
) {
    tokio::spawn(async move {
        if let Err(e) = run_archive_loop(state, telegram_client, tx.clone(), pause_flag).await {"""

new_start_archive_run = """pub fn start_archive_run(
    state: State,
    active_pair_index: usize,
    telegram_client: Arc<TelegramClient>,
    tx: mpsc::Sender<AppEvent>,
    pause_flag: Arc<std::sync::atomic::AtomicBool>,
) {
    tokio::spawn(async move {
        if let Err(e) = run_archive_loop(state, active_pair_index, telegram_client, tx.clone(), pause_flag).await {"""

code = code.replace(old_start_archive_run, new_start_archive_run)

# run_archive_loop
old_run_archive_loop = """async fn run_archive_loop(
    mut state: State,
    telegram_client: Arc<TelegramClient>,
    tx: mpsc::Sender<AppEvent>,
    pause_flag: Arc<std::sync::atomic::AtomicBool>,
) -> anyhow::Result<()> {
    let source_channel_id = state
        .source_channel_id
        .ok_or_else(|| anyhow::anyhow!("Source channel not set"))?;

    let dest_group_id = state
        .dest_group_id
        .ok_or_else(|| anyhow::anyhow!("Destination group not set"))?;"""

new_run_archive_loop = """async fn run_archive_loop(
    mut state: State,
    active_pair_index: usize,
    telegram_client: Arc<TelegramClient>,
    tx: mpsc::Sender<AppEvent>,
    pause_flag: Arc<std::sync::atomic::AtomicBool>,
) -> anyhow::Result<()> {
    if state.channel_pairs.is_empty() || state.channel_pairs[active_pair_index].source_channel_id == 0 {
        anyhow::bail!("Source channel not set");
    }
    let source_channel_id = state.channel_pairs[active_pair_index].source_channel_id;

    if state.channel_pairs[active_pair_index].dest_group_id == 0 {
        anyhow::bail!("Destination group not set");
    }
    let dest_group_id = state.channel_pairs[active_pair_index].dest_group_id;"""

code = code.replace(old_run_archive_loop, new_run_archive_loop)

# post_count_threshold and start_id
code = code.replace(
    "if state.last_forwarded_message_id.unwrap_or(0) < lowest_allowed {",
    "if state.channel_pairs[active_pair_index].last_forwarded_message_id.unwrap_or(0) < lowest_allowed {"
).replace(
    "state.last_forwarded_message_id = Some(lowest_allowed - 1);",
    "state.channel_pairs[active_pair_index].last_forwarded_message_id = Some(lowest_allowed - 1);"
).replace(
    "let start_id = match state.last_forwarded_message_id {",
    "let start_id = match state.channel_pairs[active_pair_index].last_forwarded_message_id {"
)

# state.dest_topic_id
code = code.replace(
    "state.dest_topic_id,",
    "state.channel_pairs[active_pair_index].dest_topic_id,"
)

# state.last_forwarded_message_id = Some(current_end);
code = code.replace(
    "state.last_forwarded_message_id = Some(current_end);",
    "state.channel_pairs[active_pair_index].last_forwarded_message_id = Some(current_end);"
)

with open("src/archive/mod.rs", "w") as f:
    f.write(code)
