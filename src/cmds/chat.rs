use std::sync::Arc;
use std::io;

use std::sync::atomic::{AtomicBool, Ordering};

#[actix_rt::main]
pub async fn chat() -> std::io::Result<()> {
    let mut input = String::new();
    let check  = Arc::new(AtomicBool::new(true));
    let update = check.clone();
    
    ctrlc::set_handler(move || {
        update.store(false, Ordering::SeqCst);
    }).expect("Error setting Ctrl-C handler");

    while check.load(Ordering::SeqCst) {
        println!("Enter a message:");

        io::stdin()
            .read_line(&mut input)
            .expect("Failed to read input");
    }

    Ok(())
}
