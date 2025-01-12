use juniper::GraphQLObject;

pub mod database;
pub mod tsdb;

#[derive(GraphQLObject, Debug)]
#[graphql(description = "Information about japaness candle stick")]
pub struct CandleStick {
    #[graphql(description = "timestamp")]
    pub t: i32,

    #[graphql(description = "open price")]
    pub o: f64,

    #[graphql(description = "highest price")]
    pub h: f64,

    #[graphql(description = "close price")]
    pub c: f64,

    #[graphql(description = "lowest price")]
    pub l: f64,

    #[graphql(description = "volume")]
    pub v: i32,
}

#[derive(GraphQLObject, Debug)]
#[graphql(description = "Argument to configure jobs")]
pub struct Argument {
    #[graphql(description = "argument in which will be used for jobs")]
    pub argument: String,

    #[graphql(description = "value of the argument which you are configuring")]
    pub value: String,
}

#[derive(GraphQLObject, Debug)]
#[graphql(description = "Information about cronjob")]
pub struct CronJob {
    #[graphql(description = "timeout")]
    pub timeout: i32,

    #[graphql(description = "interval when cronjob run")]
    pub interval: String,

    #[graphql(description = "which job will be perform to resolve tasks")]
    pub resolver: String,

    #[graphql(description = "arguments for this job")]
    pub arguments: Option<Vec<Argument>>,
}

#[derive(GraphQLObject, Debug)]
#[graphql(description = "Information about job")]
pub struct SingleJob {
    #[graphql(description = "timeout")]
    pub timeout: i32,

    #[graphql(description = "which job will be perform to resolve tasks")]
    pub resolver: String,

    #[graphql(description = "arguments for this job")]
    pub arguments: Option<Vec<Argument>>,

    #[graphql(description = "start of time range where timeserie data will be taken")]
    pub from: Option<i32>,

    #[graphql(description = "end of time range where timeserie data will be taken")]
    pub to: Option<i32>,
}

