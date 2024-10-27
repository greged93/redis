use eyre::Result;
use std::io::{Read, Write};
use std::net::TcpListener;

fn main() -> Result<()> {
    let listener = TcpListener::bind("127.0.0.1:6379")?;

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                let mut buffer = [0; 512];
                while let Ok(s) = stream.read(&mut buffer) {
                    println!("Read {s} bytes");
                    stream.write_all(b"+PONG\r\n")?;
                }
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }

    Ok(())
}
