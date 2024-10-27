use eyre::Result;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

#[tokio::main]
async fn main() -> Result<()> {
    let listener = TcpListener::bind("127.0.0.1:6379")?;

    let mut handles = Vec::new();
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let t = tokio::spawn(async { handle_connection(stream) });
                handles.push(t);
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }

    // Join all the handles to be sure we answer to all incoming requests
    for t in handles {
        t.await??
    }

    Ok(())
}

/// Handle a TCP stream by writing PONG to it for every input it reads from it.
fn handle_connection(mut stream: TcpStream) -> Result<()> {
    let mut buffer = [0; 512];
    while let Ok(s) = stream.read(&mut buffer) {
        println!("Read {s} bytes");
        stream.write_all(b"+PONG\r\n")?;
    }

    Ok(())
}
