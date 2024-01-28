use pembantu_core::bot::BotKind;
use teloxide::{requests::ResponseResult, types::Message, Bot};
use crate::command::Command;
use std::env;

pub struct Conversation {
    api_key: String
}

impl Conversation {
    pub fn new(bot_kind: BotKind) -> Self {
        Self {
            api_key: match bot_kind {
                BotKind::OpenRouter(api_key) => api_key
            }
        }
    }

    pub async fn reply_command(&self, bot: Bot, msg: Message, cmd: Command) -> ResponseResult<()> {
        match cmd {
            Command::Ask => {
                Ok(())
            }
        }
    }

    pub async fn reply_message(&self) {
        
    }
}


#[cfg(test)]
mod tests {
    use std::env;
    use pembantu_core::bot::Bot;
    use pembantu_core::api::openrouter::OpenRouterAPI;
    use dotenv::dotenv;
    #[actix_rt::test]
    async fn test_openrouter() {
        dotenv().ok();

        let mut api_key = env::var("OPENROUTER_API").unwrap();
        let mut api = OpenRouterAPI::new(api_key);
        
        let result = api.generate("Hi, how are you?".into()).await;
        println!("{}", result)
    }
}