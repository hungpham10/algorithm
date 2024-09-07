use std::collections::BTreeMap;

use crate::actors::fireant;
use crate::analytic::Sentiment;

pub struct Mention {}

struct SentimentTransform {
    symbol: String,
    sentiment: i32,
}

impl Mention {
    pub fn new() -> Self {
        Self {}
    }

    pub fn count_mention_by_symbol(
        &self,
        result: &mut BTreeMap<String, Sentiment>,
        posts: &Vec<fireant::Post>,
    ) {
        posts
            .iter()
            .map(move |post| {
                post.taggedSymbols
                    .iter()
                    .map(move |tag| tag.symbol.clone())
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>()
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .into_iter()
            .map(move |symbol| {
                result
                    .entry(symbol.clone())
                    .or_insert(Sentiment::new())
                    .mention += 1;
            })
            .collect::<Vec<_>>();
    }

    pub fn count_sentiment_vote_by_symbol(
        &self,
        result: &mut BTreeMap<String, Sentiment>,
        posts: &Vec<fireant::Post>,
    ) {
        posts
            .iter()
            .map(move |post| {
                post.taggedSymbols
                    .iter()
                    .map(move |tag| SentimentTransform {
                        symbol: tag.symbol.clone(),
                        sentiment: post.sentiment,
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>()
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .into_iter()
            .map(move |comment| {
                if comment.sentiment > 0 {
                    result
                        .entry(comment.symbol.clone())
                        .or_insert(Sentiment::new())
                        .votes
                        .positive += 1;
                }
                if comment.sentiment < 0 {
                    result
                        .entry(comment.symbol.clone())
                        .or_insert(Sentiment::new())
                        .votes
                        .negative += 1;
                }
                if comment.sentiment == 0 {
                    result
                        .entry(comment.symbol.clone())
                        .or_insert(Sentiment::new())
                        .votes
                        .neutral += 1;
                }
            })
            .collect::<Vec<_>>();
    }
}
