pub mod dist_mutex;
mod network;
pub mod packet_dispatcher;

#[actix_rt::main]
async fn main() {
    println!("Hello, world!");
}
