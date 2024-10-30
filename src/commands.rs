use crate::parser::Value;
use miette::miette;

/// The available commands for the Redis client
#[derive(PartialEq, Clone, Debug)]
pub enum RedisCommands {
    Ping,
    Echo(String),
}

impl TryFrom<Value> for RedisCommands {
    type Error = miette::Error;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            // Parse a list of command + args
            Value::Array(values) => {
                let command = values
                    .first()
                    .and_then(Value::to_string)
                    .ok_or_else(|| miette!("not a command"))?;
                match command.to_lowercase().as_str() {
                    "ping" => Ok(Self::Ping),
                    "echo" => Ok(Self::Echo(
                        values
                            .get(1)
                            .and_then(|val| val.encode().ok())
                            .ok_or_else(|| miette!("missing echo argument"))?,
                    )),
                    x => Err(miette!("expected commend, got {x}")),
                }
            }
            _ => Err(miette!("incorrect command")),
        }
    }
}
