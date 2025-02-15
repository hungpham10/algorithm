use std::collections::BTreeMap;

use crate::actors::fireant;
use crate::analytic::Sentiment;

pub struct Mention {
}

struct SentimentTransform {
    symbol: String,
    sentiment: i32,
}

struct LinkTransform {
    symbol: String,
    have_link: bool,
}

impl Mention {
    pub fn new() -> Self {
        Self {}
    }

    pub fn count_youtube_link_by_symbol(
        &self,
        result: &mut BTreeMap<String, Sentiment>,
        posts: &Vec<fireant::Post>,
    ) {
        let _ = posts
            .iter()
            .map(move |post| {
                let have_link = match &post.link {
                    Some(link) => link.len() > 0,
                    None => false,
                };

                post.taggedSymbols
                    .iter()
                    .map(move |tag| LinkTransform {
                        have_link,
                        symbol: tag.symbol.clone(),
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>()
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .into_iter()
            .map(move |link| {
                if link.have_link {
                    result
                        .entry(link.symbol.clone())
                        .or_insert(Sentiment::new())
                        .promotion += 1;
                }
            })
            .collect::<Vec<_>>();
    }

    pub fn count_mention_by_symbol(
        &self,
        result: &mut BTreeMap<String, Sentiment>,
        posts: &Vec<fireant::Post>,
    ) {
        let _ = posts
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
        let _ = posts
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
