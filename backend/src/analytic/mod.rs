use juniper::GraphQLObject;
use serde::{Deserialize, Serialize};

#[derive(GraphQLObject, Serialize, Deserialize, Clone, Debug)]
#[graphql(description = "Count number of vote per sentiment comments from users")]
pub struct Voting {
    pub positive: i32,
    pub neutral: i32,
    pub negative: i32,
}

#[derive(GraphQLObject, Serialize, Deserialize, Clone, Debug)]
#[graphql(description = "Information sentiment analysis")]
pub struct Sentiment {
    #[graphql(description = "Number of mention in forum")]
    pub mention: i32,

    #[graphql(description = "Vote couting replying by user")]
    pub votes: Voting,

    #[graphql(description = "")]
    pub promotion: i32,
}

impl Sentiment {
    fn new() -> Self {
        Self {
            votes: Voting {
                positive: 0,
                neutral: 0,
                negative: 0,
            },
            promotion: 0,
            mention: 0,
        }
    }
}

pub mod mention;
