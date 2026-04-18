use grammers_client::Client;
fn check_upload(client: &Client) {
    let _ = client.upload_file("test");
}
