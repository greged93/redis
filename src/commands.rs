/// The available commands for the Redis client
#[derive(PartialEq, Clone, Debug)]
pub enum RedisCommands {
    Ping,
    Echo,
}
