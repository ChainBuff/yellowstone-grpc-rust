mod common;
mod subscribe_logs;
mod subscribe_tx;
mod subscribe_instructions;

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    println!("Hello, world!");
}
