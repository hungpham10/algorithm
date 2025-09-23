pub mod facebook;

pub struct Slack {
    pub token: String,
}

pub struct Facebook {
    pub token: String,
    pub incomming_secret: String,
    pub outgoing_secret: String,
}

pub struct Chat {
    pub slack: Slack,
    pub fb: Facebook,
}
