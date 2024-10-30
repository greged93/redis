use miette::{miette, Result};
use redis_starter_rust::commands::RedisCommands;
use redis_starter_rust::parser::RedisParser;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

#[tokio::main]
async fn main() -> Result<()> {
    let listener = TcpListener::bind("127.0.0.1:6379").map_err(|e| miette!(e))?;

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
        t.await.map_err(|e| miette!(e))??
    }

    Ok(())
}

/// Handle a TCP stream connection.
fn handle_connection(mut stream: TcpStream) -> Result<()> {
    let mut buffer = [0; 512];
    while let Ok(s) = stream.read(&mut buffer) {
        println!("Read {s} bytes");
        let mut parser = RedisParser::new(&buffer[..s]);
        let command: RedisCommands = parser
            .next()
            .ok_or_else(|| miette!("empty input"))??
            .try_into()?;
        match command {
            RedisCommands::Ping => stream.write_all(b"+PONG\r\n"),
            RedisCommands::Echo(x) => stream.write_all(x.as_bytes()),
        }
        .map_err(|e| miette!(e))?
    }

    Ok(())
}
