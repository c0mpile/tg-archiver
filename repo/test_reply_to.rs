use grammers_client::message::InputMessage;

fn main() {
    let msg = InputMessage::new().reply_to(Some(123));
    println!("{:?}", msg);
}
